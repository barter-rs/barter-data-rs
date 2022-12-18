#![warn(
    // missing_debug_implementations,
    missing_copy_implementations,
    rust_2018_idioms,
    // missing_docs
)]

///! # Barter-Data
use crate::{
    exchange::Connector,
    model::Market,
    subscriber::{
        subscription::{SubKind, Subscription},
        Subscriber,
    },
    transformer::ExchangeTransformer,
};
use async_trait::async_trait;
use barter_integration::{
    error::SocketError,
    protocol::websocket::{WebSocketParser, WsStream},
    ExchangeStream,
};
use futures::{SinkExt, Stream, StreamExt};
use tokio::sync::mpsc;
use tracing::error;
use barter_integration::protocol::websocket::{WsMessage, WsSink};
use crate::exchange::ExchangeId;

pub mod exchange;
/// Todo:
pub mod model;
pub mod subscriber;
pub mod transformer;

// Todo: Thoughts
//  - May sure Connector is protocol agnostic

// Todo:

// Todo: Nice To Have:
//  - Clean up distribution of responses to the exchange... it's messy.
//  - Add Pong strategy so StatelessTransformer can be used ubiquitously.

// Todo: Before Release:
//  - Fix imports
//  - Add derives eagerly
//  - Rust docs
//  - Check rust docs & fix
//  - Add unit tests from develop branch, etc.

/// Convenient type alias for an [`ExchangeStream`] utilising a tungstenite [`WebSocket`]
pub type ExchangeWsStream<Transformer> = ExchangeStream<WebSocketParser, WsStream, Transformer>;

/// Defines a generic identification type for the implementor.
pub trait Identifier<T> {
    fn id(&self) -> T;
}

/// Defines the [`MarketStream`] kind associated with each exchange [`Subscription`] [`SubKind`].
///
/// ### Example: Subscription<Coinbase, PublicTrades>
/// ` Stream = ExchangeWsStream<StatelessTransformer<Self, PublicTrades, CoinbaseTrade>>`
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
        tokio::spawn(send_messages_to_exchange(Exchange::ID, ws_sink, ws_sink_rx));

        // Construct Transformer associated with this Exchange and SubKind
        let transformer = Transformer::new(ws_sink_tx, map);

        Ok(ExchangeWsStream::new(ws_stream, transformer))
    }
}

/// Receive [`WsMessage`]s transmitted from the [`ExchangeTransformer`] and send them to the
/// exchange via the [`WsSink`].
///
/// Note:
/// ExchangeTransformer is operating in a synchronous trait context so we use this separate task
/// to avoid adding `#[\async_trait\]` to the transformer - this avoids allocations.
async fn send_messages_to_exchange(
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