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
use futures::{Stream, StreamExt};
use tokio::sync::mpsc;

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

/// Todo:
pub trait Identifier<T> {
    fn id(&self) -> T;
}

/// Todo:
pub trait StreamSelector<Kind>
where
    Self: Connector,
    Kind: SubKind,
{
    type Stream: MarketStream<Self, Kind>;
}

/// Todo:
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
        // Todo:
        let (_, ws_stream) = websocket.split();
        let (ws_sink_tx, _) = mpsc::unbounded_channel();

        // Construct Transformer associated with this Exchange and SubKind
        let transformer = Transformer::new(ws_sink_tx, map);

        Ok(ExchangeWsStream::new(ws_stream, transformer))
    }
}
