#![warn(
    missing_debug_implementations,
    missing_copy_implementations,
    rust_2018_idioms,
    // missing_docs
)]

use crate::model::{MarketEvent, Subscription, SubscriptionIds, SubscriptionMeta};
use async_trait::async_trait;
use barter_integration::{
    error::SocketError,
    model::Exchange,
    protocol::websocket::{connect, WebSocket, WebSocketParser, WsMessage, WsSink, WsStream},
    Event, ExchangeStream, Transformer, Validator,
};
use futures::{SinkExt, Stream, StreamExt};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::{
    fmt::{Debug, Display, Formatter},
    time::Duration,
};
use tokio::sync::mpsc;
use tracing::error;

///! # Barter-Data

/// Core data structures to support consuming [`MarketStream`]s.
///
/// eg/ `MarketEvent`, `PublicTrade`, etc.
pub mod model;

/// Contains `Subscriber` & `ExchangeMapper` implementations for specific exchanges.
pub mod exchange;

/// Initialises [`MarketStream`]s for an arbitrary number of exchanges using generic Barter
/// [`Subscription`]s.
pub mod builder;

/// Convenient type alias for an [`ExchangeStream`] utilising a tungstenite [`WebSocket`]
pub type ExchangeWsStream<Exchange> =
    ExchangeStream<WebSocketParser, WsStream, Exchange, MarketEvent>;

/// [`Stream`] supertrait for streams that yield [`MarketEvent`]s. Provides an entry-point abstraction
/// for an [`ExchangeStream`].
#[async_trait]
pub trait MarketStream:
    Stream<Item = Result<Event<MarketEvent>, SocketError>> + Sized + Unpin
{
    /// Initialises a new [`MarketStream`] using the provided subscriptions.
    async fn init(subscriptions: &[Subscription]) -> Result<Self, SocketError>;
}

/// Trait that defines how a subscriber will establish a [`WebSocket`] connection with an exchange,
/// and action [`Subscription`]s. This must be implemented when integrating a new exchange.
#[async_trait]
pub trait Subscriber {
    /// Deserialisable type that this [`Subscriber`] expects to receive from the exchange in
    /// response to [`Subscription`] requests. Implements [`Validator`] in order to determine
    /// if the `SubResponse` communicates a successful outcome.
    type SubResponse: Validator + DeserializeOwned;

    /// Initialises a [`WebSocket`] connection, actions the provided collection of Barter
    /// [`Subscription`]s, and validates that the [`Subscription`] were accepted by the exchange.
    async fn subscribe(
        subscriptions: &[Subscription],
    ) -> Result<(WebSocket, SubscriptionIds), SocketError> {
        // Connect to exchange
        let mut websocket = connect(Self::base_url()).await?;

        // Subscribe
        let SubscriptionMeta {
            ids,
            subscriptions,
            expected_responses,
        } = Self::build_subscription_meta(subscriptions)?;

        for subscription in subscriptions {
            websocket.send(subscription).await?;
        }

        // Validate subscriptions
        let ids = Self::validate(ids, &mut websocket, expected_responses).await?;

        Ok((websocket, ids))
    }

    /// Returns the Base URL of the exchange to establish a connection with.
    fn base_url() -> &'static str;

    /// Uses the provided Barter [`Subscription`]s to build exchange specific subscription
    /// payloads. Generates a [`SubscriptionIds`] `Hashmap` that is used by an [`ExchangeTransformer`]
    /// to identify the Barter [`Subscription`]s associated with received messages.
    fn build_subscription_meta(
        subscriptions: &[Subscription],
    ) -> Result<SubscriptionMeta, SocketError>;

    /// Uses the provided [`WebSocket`] connection to consume [`Subscription`] responses and
    /// validate their outcomes.
    async fn validate(
        ids: SubscriptionIds,
        websocket: &mut WebSocket,
        expected_responses: usize,
    ) -> Result<SubscriptionIds, SocketError> {
        // Establish time limit in which we expect to validate all the Subscriptions
        let timeout = Self::subscription_timeout();

        // Parameter to keep track of successful Subscription outcomes
        let mut success_responses = 0usize;

        loop {
            // Break if all Subscriptions were a success
            if success_responses == expected_responses {
                break Ok(ids);
            }

            tokio::select! {
                // If timeout reached, return SubscribeError
                _ = tokio::time::sleep(timeout) => {
                    break Err(SocketError::Subscribe(
                        format!("subscription validation timeout reached: {:?}", timeout))
                    )
                },

                // Parse incoming messages and determine subscription outcomes
                message = websocket.next() => match message {
                    Some(Ok(WsMessage::Text(payload))) => {
                        if let Ok(response) = serde_json::from_str::<Self::SubResponse>(&payload) {
                            match response.validate() {
                                // Subscription success
                                Ok(_) => { success_responses += 1; }

                                // Subscription failure
                                Err(err) => break Err(err)
                            }
                        } else {
                            // Some already active Subscriptions may start coming through
                            continue;
                        }
                    },
                    Some(Ok(WsMessage::Close(close_frame))) => {
                        break Err(SocketError::Subscribe(
                            format!("received WebSocket CloseFrame: {:?}", close_frame))
                        )
                    },
                    _ => continue,
                }
            }
        }
    }

    /// Return the expected `Duration` in which the exchange will respond to all actioned
    /// `WebSocket` [`Subscription`] requests.
    ///
    /// Default: 10 seconds
    fn subscription_timeout() -> Duration {
        Duration::from_secs(10)
    }
}

/// Defines how to translate between exchange specific data structures & Barter data
/// structures. This must be implemented when integrating a new exchange.
pub trait ExchangeTransformer: Transformer<MarketEvent> + Sized {
    /// Unique identifier for an [`ExchangeTransformer`].
    const EXCHANGE: ExchangeId;

    /// Constructs a new [`ExchangeTransformer`] using a transmitter to the [`WsSink`] and the
    /// [`SubscriptionIds`] `HashMap`.
    ///
    /// Note:
    ///  - If required, the [`WsSink`] transmitter may be used to send messages to the exchange.
    fn new(ws_sink_tx: mpsc::UnboundedSender<WsMessage>, ids: SubscriptionIds) -> Self;
}

#[async_trait]
impl<Exchange> MarketStream for ExchangeWsStream<Exchange>
where
    Exchange: Subscriber + ExchangeTransformer + Send,
{
    async fn init(subscriptions: &[Subscription]) -> Result<Self, SocketError> {
        // Connect & subscribe
        let (websocket, ids) = Exchange::subscribe(subscriptions).await?;

        // Split WebSocket into WsStream & WsSink components
        let (ws_sink, ws_stream) = websocket.split();

        // Task to distribute ExchangeTransformer outgoing messages (eg/ custom pongs) to exchange
        // --> ExchangeTransformer is operating in a synchronous trait context
        // --> ExchangeTransformer sends messages sync via channel to async distribution task
        // --> Async distribution tasks forwards the messages to the exchange via the ws_sink
        let (ws_sink_tx, ws_sink_rx) = mpsc::unbounded_channel();
        tokio::spawn(distribute_responses_to_the_exchange(
            Exchange::EXCHANGE,
            ws_sink,
            ws_sink_rx,
        ));

        // Construct ExchangeTransformer w/ transmitter to WsSink
        let transformer = Exchange::new(ws_sink_tx, ids);

        Ok(ExchangeWsStream::new(ws_stream, transformer))
    }
}

/// Used to uniquely identify an [`ExchangeTransformer`] implementation. Each variant represents an
/// exchange server which can be subscribed to. Note that an exchange may have multiple servers
/// (eg/ binance, binance_futures), therefore there could be a many-to-one relationship between
/// an [`ExchangeId`] and an [`Exchange`].
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Deserialize, Serialize)]
#[serde(rename = "exchange", rename_all = "snake_case")]
pub enum ExchangeId {
    BinanceFuturesUsd,
    Binance,
    Coinbase,
    Ftx,
    Kraken,
    Bitfinex,
    /// Kucoin spot implementation
    Kucoin,
}

impl From<ExchangeId> for Exchange {
    fn from(exchange_id: ExchangeId) -> Self {
        Exchange::from(exchange_id.as_str())
    }
}

impl Display for ExchangeId {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl ExchangeId {
    /// Return the exchange name associated with this [`ExchangeId`].
    ///
    /// eg/ ExchangeId::BinanceFuturesUsd => "binance"
    pub fn name(&self) -> &'static str {
        match self {
            ExchangeId::Binance | ExchangeId::BinanceFuturesUsd => "binance",
            ExchangeId::Coinbase => "coinbase",
            ExchangeId::Ftx => "ftx",
            ExchangeId::Kraken => "kraken",
            ExchangeId::Bitfinex => "bitfinex",
            ExchangeId::Kucoin => "kucoin",
        }
    }

    /// Return the &str representation this [`ExchangeId`] is associated with.
    pub fn as_str(&self) -> &'static str {
        match self {
            ExchangeId::Binance => "binance",
            ExchangeId::BinanceFuturesUsd => "binance_futures_usd",
            ExchangeId::Coinbase => "coinbase",
            ExchangeId::Ftx => "ftx",
            ExchangeId::Kraken => "kraken",
            ExchangeId::Bitfinex => "bitfinex",
            ExchangeId::Kucoin => "kucoin",
        }
    }

    /// Determines whether this [`ExchangeId`] supports the ingestion of
    /// [`InstrumentKind::Spot`](barter_integration::model::InstrumentKind) market data.
    pub fn supports_spot(&self) -> bool {
        match self {
            ExchangeId::BinanceFuturesUsd => false,
            _ => true,
        }
    }

    /// Determines whether this [`ExchangeId`] supports the collection of
    /// [`InstrumentKind::Future**`](barter_integration::model::InstrumentKind) market data.
    pub fn supports_futures(&self) -> bool {
        match self {
            ExchangeId::BinanceFuturesUsd => true,
            ExchangeId::Ftx => true,
            _ => false,
        }
    }

    /// Determines whether this [`ExchangeId`] supports the collection of
    /// [`PublicTrade`](model::PublicTrade) market data.
    pub fn supports_trades(&self) -> bool {
        match self {
            _ => true,
        }
    }

    /// Determines whether this [`ExchangeId`] supports the collection of
    /// [`Candle`](model::Candle) market data.
    pub fn supports_candles(&self) -> bool {
        match self {
            ExchangeId::Kraken => true,
            ExchangeId::Bitfinex => true,
            _ => false,
        }
    }

    /// Determines whether this [`ExchangeId`] supports the collection of
    /// L2 OrderBook market data.
    pub fn supports_order_books(&self) -> bool {
        match self {
            _ => false,
        }
    }
}

/// Consume [`WsMessage`]s transmitted from the [`ExchangeTransformer`] and send them on to the
/// exchange via the [`WsSink`].
///
/// If an [`ExchangeTransformer`] is required to send responses to the exchange (eg/ custom pongs),
/// it can so by transmitting the responses to the  `mpsc::UnboundedReceiver<WsMessage>` owned by
/// this asynchronous distribution task. These are then sent to the exchange via the [`WsSink`].
/// This is required because an [`ExchangeTransformer`] is operating in a synchronous trait context,
/// and therefore cannot flush the [`WsSink`] without the [`futures:task::context`].
async fn distribute_responses_to_the_exchange(
    exchange: ExchangeId,
    mut ws_sink: WsSink,
    mut ws_sink_rx: mpsc::UnboundedReceiver<WsMessage>,
) {
    while let Some(message) = ws_sink_rx.recv().await {
        if let Err(error) = ws_sink.send(message).await {
            if barter_integration::protocol::websocket::is_websocket_disconnected(&error) {
                break;
            }

            // Log error only if WsMessage failed to send over a connected WebSocket
            error!(
                %exchange,
                %error,
                "failed to send ExchangeTransformer output message to the exchange via WsSink"
            );
        }
    }
}

/// Test utilities for conveniently generating public [`MarketEvent`] types.
pub mod test_util {
    use crate::{
        model::{Candle, DataKind, MarketEvent, PublicTrade},
        ExchangeId,
    };
    use barter_integration::model::{Exchange, Instrument, InstrumentKind, Side};
    use chrono::Utc;
    use std::ops::{Add, Sub};

    /// Build a [`MarketEvent`] of [`DataKind::Trade`] with the provided [`Side`].
    pub fn market_trade(side: Side) -> MarketEvent {
        MarketEvent {
            exchange_time: Utc::now(),
            received_time: Utc::now(),
            exchange: Exchange::from(ExchangeId::Binance),
            instrument: Instrument::from(("btc", "usdt", InstrumentKind::Spot)),
            kind: DataKind::Trade(PublicTrade {
                id: "trade_id".to_string(),
                price: 1000.0,
                quantity: 1.0,
                side,
            }),
        }
    }

    /// Build a [`MarketEvent`] of [`DataKind::Candle`] with the provided time interval.
    pub fn market_candle(interval: chrono::Duration) -> MarketEvent {
        let now = Utc::now();
        MarketEvent {
            exchange_time: now,
            received_time: now.add(chrono::Duration::milliseconds(200)),
            exchange: Exchange::from(ExchangeId::Binance),
            instrument: Instrument::from(("btc", "usdt", InstrumentKind::Spot)),
            kind: DataKind::Candle(Candle {
                start_time: now.sub(interval),
                end_time: now,
                open: 960.0,
                high: 1100.0,
                low: 950.0,
                close: 1000.0,
                volume: 100000.0,
                trade_count: 1000,
            }),
        }
    }
}
