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
    model::{Exchange, SubscriptionId},
    protocol::websocket::{connect, WebSocket, WebSocketParser, WsMessage},
    Event, ExchangeSocket, Transformer,
};
use futures::{SinkExt, Stream, StreamExt};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::{
    fmt::{Debug, Display, Formatter},
    time::Duration,
};

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

/// Convenient type alias for an [`ExchangeSocket`] utilising a tungstenite [`WebSocket`]
pub type ExchangeWebSocket<Exchange> =
    ExchangeSocket<WebSocketParser, WebSocket, Exchange, MarketEvent>;

/// [`Stream`] supertrait for streams that yield [`MarketEvent`]s. Provides an entry-point
/// abstraction for an [`ExchangeSocket`].
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
        Self::validate(&mut websocket, expected_responses).await?;

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
        websocket: &mut WebSocket,
        expected_responses: usize,
    ) -> Result<(), SocketError> {
        // Establish time limit in which we expect to validate all the Subscriptions
        let timeout = Self::subscription_timeout();

        // Parameter to keep track of successful Subscription outcomes
        let mut success_responses = 0usize;

        loop {
            // Break if all Subscriptions were a success
            if success_responses == expected_responses {
                break Ok(());
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

/// [`Validator`]s are capable of determining if their internal state is satisfactory to fulfill
/// some use case defined by the implementor.
pub trait Validator {
    /// Check if `Self` is valid for some use case.
    fn validate(self) -> Result<Self, SocketError>
    where
        Self: Sized;
}

/// defines how to translate between exchange specific data structures & Barter data
/// structures. This must be implemented when integrating a new exchange.
pub trait ExchangeTransformer: Transformer<MarketEvent> + Sized
where
    <Self as Transformer<MarketEvent>>::Input: Identifiable,
{
    /// Unique identifier for an [`ExchangeTransformer`].
    const EXCHANGE: ExchangeId;

    /// Construct a new [`ExchangeTransformer`] using a `HashMap` containing the relationship between
    /// all active Barter [`Subscription`]s and their associated exchange specific identifiers.
    fn new(ids: SubscriptionIds) -> Self;
}

/// [`Identifiable`] structures are capable of determining their associated [`SubscriptionId`]. Used
/// by [`ExchangeTransformer`] implementations to determine the original Barter [`Subscription`]
/// associated with an incoming exchange message.
pub trait Identifiable {
    fn id(&self) -> SubscriptionId;
}

#[async_trait]
impl<Exchange> MarketStream for ExchangeWebSocket<Exchange>
where
    Exchange: Subscriber + ExchangeTransformer + Send,
    <Exchange as Transformer<MarketEvent>>::Input: Identifiable,
{
    async fn init(subscriptions: &[Subscription]) -> Result<Self, SocketError> {
        // Connect & subscribe
        let (websocket, ids) = Exchange::subscribe(subscriptions).await?;

        // Construct ExchangeTransformer
        let transformer = Exchange::new(ids);

        Ok(ExchangeSocket::new(websocket, transformer))
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
    Ftx,
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
            ExchangeId::Ftx => "ftx",
        }
    }

    /// Return the &str representation this [`ExchangeId`] is associated with.
    pub fn as_str(&self) -> &'static str {
        match self {
            ExchangeId::Binance => "binance",
            ExchangeId::BinanceFuturesUsd => "binance_futures_usd",
            ExchangeId::Ftx => "ftx",
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
}

/// Test utilities for conveniently generating public [`MarketEvent`] types.
pub mod test_util {
    use crate::{
        model::{DataKind, MarketEvent, PublicTrade},
        ExchangeId,
    };
    use barter_integration::model::{Exchange, Instrument, InstrumentKind, Side};
    use chrono::Utc;

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
}
