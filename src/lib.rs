#![warn(
    // missing_debug_implementations,
    missing_copy_implementations,
    rust_2018_idioms,
    // missing_docs
)]

///! # Barter-Data
use crate::{
    event::Market,
    exchange::{Connector, ExchangeId, PingInterval},
    subscriber::Subscriber,
    subscription::{SubKind, Subscription},
    transformer::ExchangeTransformer,
};
use async_trait::async_trait;
use barter_integration::{
    error::SocketError,
    protocol::websocket::{WebSocketParser, WsMessage, WsSink, WsStream},
    ExchangeStream,
};
use futures::{SinkExt, Stream, StreamExt};
use tokio::sync::mpsc;
use tracing::{debug, error};

/// Todo:
pub mod event;
pub mod exchange;
pub mod subscriber;
pub mod subscription;
pub mod transformer;
pub mod book;

// Todo:
//  - SubscriptionId<T> ? eg/ Bitfinex uses u32 channel_id
//  - Use Cow for SubscriptionId? Would stop cloning deserialised data eg/ market since SubscriptionId just used for SubMap.get()
//  - Build front end api for new Subscription<Exchange, SubKind> paradigm.
//  - Add validation for Subscriptions at Connector and StreamBuilder level. Ideally at compile time.
//   '--> feels like impl Identifier<GateioChannel> for Subscription is a missed opportunity
//   '--> This Sub compiles: (GateioFuturesUsd::default(), "btc", "usdt", InstrumentKind::Spot, PublicTrades).into(),
//  - Add aliases:
//    - Kraken btc -> xbt ("btc" sub accepted but trades use XBT so it's unidentifiable)
//    - Bitfinex ust -> usdt

// Todo: Before Release:
//  - Add logging - ensure all facets are the same (eg/ exchange instead of exchange_id)
//  - Fix imports
//  - Add derives eagerly
//  - Rust docs
//  - Check rust docs & fix
//  - Add unit tests from develop branch, etc.
//  - Add links & tags to release MR with reference to Shayan's contributions (link MR etc)

/// Convenient type alias for an [`ExchangeStream`] utilising a tungstenite [`WebSocket`]
pub type ExchangeWsStream<Transformer> = ExchangeStream<WebSocketParser, WsStream, Transformer>;

/// Defines a generic identification type for the implementor.
pub trait Identifier<T> {
    fn id(&self) -> T;
}

/// Defines the [`MarketStream`] kind associated with an exchange [`Subscription`] [`SubKind`].
pub trait StreamSelector<Kind>
where
    Self: Connector,
    Kind: SubKind,
{
    type Stream: MarketStream<Self, Kind>;
}

/// [`Stream`] that yields [`Market<Kind>`](Market) events. The type of [`Market<Kind>`](Market)
/// depends on the provided [`SubKind`] of the passed [`Subscription`]s.
#[async_trait]
pub trait MarketStream<Exchange, Kind>
where
    Self: Stream<Item = Result<Market<Kind::Event>, SocketError>> + Sized + Unpin,
    Exchange: Connector,
    Kind: SubKind,
{
    async fn init(subscriptions: &[Subscription<Exchange, Kind>]) -> Result<Self, SocketError>
    where
        Subscription<Exchange, Kind>: Identifier<Exchange::Channel> + Identifier<Exchange::Market>;
}

#[async_trait]
impl<Exchange, Kind, Transformer> MarketStream<Exchange, Kind> for ExchangeWsStream<Transformer>
where
    Exchange: Connector + Send + Sync,
    Kind: SubKind + Send + Sync,
    Transformer: ExchangeTransformer<Exchange, Kind>,
{
    async fn init(subscriptions: &[Subscription<Exchange, Kind>]) -> Result<Self, SocketError>
    where
        Subscription<Exchange, Kind>: Identifier<Exchange::Channel> + Identifier<Exchange::Market>,
    {
        // Connect & subscribe
        let (websocket, map) = Exchange::Subscriber::subscribe(subscriptions).await?;

        // Split WebSocket into WsStream & WsSink components
        let (ws_sink, ws_stream) = websocket.split();

        // Spawn task to distribute Transformer messages (eg/ custom pongs) to the exchange
        let (ws_sink_tx, ws_sink_rx) = mpsc::unbounded_channel();
        tokio::spawn(distribute_messages_to_exchange(
            Exchange::ID,
            ws_sink,
            ws_sink_rx,
        ));

        // Spawn optional task to distribute custom application-level pings to the exchange
        if let Some(ping_interval) = Exchange::ping_interval() {
            tokio::spawn(schedule_pings_to_exchange(
                Exchange::ID,
                ws_sink_tx.clone(),
                ping_interval,
            ));
        }

        // Construct Transformer associated with this Exchange and SubKind
        let transformer = Transformer::new(ws_sink_tx, map).await?;

        Ok(ExchangeWsStream::new(ws_stream, transformer))
    }
}

/// Receive [`WsMessage`]s transmitted from the [`ExchangeTransformer`] and distribute them to the
/// exchange via the [`WsSink`].
///
/// **Note:**
/// ExchangeTransformer is operating in a synchronous trait context so we use this separate task
/// to avoid adding `#[\async_trait\]` to the transformer - this avoids allocations.
pub async fn distribute_messages_to_exchange(
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
                "failed to send  output message to the exchange via WsSink"
            );
        }
    }
}

/// Schedule the sending of custom application-level ping [`WsMessage`]s to the exchange using
/// the provided [`PingInterval`].
///
/// **Notes:**
///  - This is only used for those exchanges that require custom application-level pings.
///  - This is additional to the protocol-level pings already handled by `tokio_tungstenite`.
pub async fn schedule_pings_to_exchange(
    exchange: ExchangeId,
    ws_sink_tx: mpsc::UnboundedSender<WsMessage>,
    PingInterval { mut interval, ping }: PingInterval,
) {
    loop {
        // Wait for next scheduled ping
        interval.tick().await;

        // Construct exchange custom application-level ping payload
        let payload = ping();
        debug!(%exchange, %payload, "sending custom application-level ping to exchange");

        if ws_sink_tx.send(payload).is_err() {
            break;
        }
    }
}
