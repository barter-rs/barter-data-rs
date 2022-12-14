#![warn(
    // missing_debug_implementations,
    missing_copy_implementations,
    rust_2018_idioms,
    // missing_docs
)]

///! # Barter-Data

use crate::{
    exchange::{Connector, ExchangeId},
    model::Market,
    subscriber::{Subscriber, subscription::{SubKind, Subscription}},
};
use barter_integration::{
    error::SocketError,
    ExchangeStream,
    protocol::websocket::{WebSocketParser, WsMessage, WsSink, WsStream},
};
use async_trait::async_trait;
use futures::{SinkExt, Stream, StreamExt};
use tokio::sync::mpsc;
use tracing::error;
///! # Barter-Data

use transformer::TransformerConstructor;

/// Todo:
pub mod model;
pub mod subscriber;
pub mod exchange;
pub mod transformer;

/// Convenient type alias for an [`ExchangeStream`] utilising a tungstenite [`WebSocket`]
pub type ExchangeWsStream<Transformer> = ExchangeStream<WebSocketParser, WsStream, Transformer>;

/// Todo:
pub trait Identifier<T> {
    fn id(&self) -> T;
}

// Todo:
//  - I don't think I need Exchange in the StatelessTransformer generics? Or perhaps I can remove
//    one of the others since I now have Exchange...
//    '--> must be a way to go from Exchange, Kind -> CoinbaseTrades
//  - There is some optimisation to do now that we have Exchange & ExchangeEvent hanging around.
//    '--> Should be able to determine the ExchangeEvent from the Exchange & Kind :)

// Todo: Nice To Have:
//  - Clean up distribution of responses to the exchange... it's messy.
//  - Add Pong strategy so StatelessTransformer can be used ubiquitously.

// Todo: Before Release:
//  - Fix imports
//  - Add derives eagerly
//  - Rust docs
//  - Check rust docs & fix
//  - Add unit tests from develop branch, etc.



/// [`Stream`] that yields [`Market<T>`] events. Type of [`Market<T>`] depends on the provided
/// [`SubKind`] of the passed [`Subscription`]s.
#[async_trait]
pub trait MarketStream<Exchange, Kind>:
where
    Self: Stream<Item = Result<Market<Kind::Event>, SocketError>> + Sized + Unpin,
    Exchange: Connector<Kind> + TransformerConstructor<Kind>,
    Kind: SubKind,
{
    /// Initialises a new [`MarketStream`] using the provided subscriptions.
    async fn init(subscriptions: &[Subscription<Exchange, Kind>]) -> Result<Self, SocketError>
    where
        Subscription<Exchange, Kind>: Identifier<Exchange::Channel> + Identifier<Exchange::Market>;
}

#[async_trait]
impl<Exchange, Kind> MarketStream<Exchange, Kind> for ExchangeWsStream<Exchange::Transformer>
where
    Exchange: Connector<Kind> + TransformerConstructor<Kind> + Send + Sync,
    Kind: SubKind + Send + Sync,
{
    async fn init(subscriptions: &[Subscription<Exchange, Kind>]) -> Result<Self, SocketError>
    where
        Subscription<Exchange, Kind>: Identifier<Exchange::Channel> + Identifier<Exchange::Market>
    {
        // Connect & subscribe
        let (
            websocket,
            map
        ) = Exchange::Subscriber::subscribe::<Exchange, Kind>(subscriptions).await?;

        // Split WebSocket into WsStream & WsSink components
        let (ws_sink, ws_stream) = websocket.split();

        // Todo: distribute messages...
        let (ws_sink_tx, ws_sink_rx) = mpsc::unbounded_channel();

        // Construct Transformer associated with this Exchange and SubKind
        let transformer = Exchange::transformer(ws_sink_tx, map);

        Ok(ExchangeWsStream::new(ws_stream, transformer))
    }
}

/// Todo:
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