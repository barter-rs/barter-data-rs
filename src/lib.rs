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
    transformer::ExchangeTransformer,
    subscriber::{
        // Subscriber,
        subscription::{SubKind, Subscription}
    },
};
use barter_integration::{error::SocketError, protocol::websocket::{WebSocketParser, WsStream}, ExchangeStream};
use async_trait::async_trait;
use futures::{Stream, StreamExt};
use tokio::sync::mpsc;


/// Todo:
pub mod model;
pub mod subscriber;
pub mod exchange;
pub mod transformer;

// Todo: Defining principles are:
//  - WE ONLY NEED EXCHANGE & KIND GENERICS TO GENERATE EVERYTHING

/// Convenient type alias for an [`ExchangeStream`] utilising a tungstenite [`WebSocket`]
pub type ExchangeWsStream<Transformer> = ExchangeStream<WebSocketParser, WsStream, Transformer>;

/// Todo:
pub trait Identifier<T> {
    fn id(&self) -> T;
}

// Todo: Add ExchangeIdentifier trait again?

pub trait StreamSelector<Kind>
where
    Self: Sized,
    Kind: SubKind,
{
    type Stream: MarketStream<Self, Kind>;
}

#[async_trait]
pub trait MarketStream<Exchange, Kind>
where
    Self: Stream<Item = Result<Market<Kind::Event>, SocketError>> + Sized + Unpin,
    Kind: SubKind,
{
    async fn init(subscriptions: &[Subscription<Exchange, Kind>]) -> Result<Self, SocketError>;
}

#[async_trait]
impl<Exchange, Kind, Transformer> MarketStream<Exchange, Kind> for ExchangeWsStream<Transformer>
where
    Exchange: Connector + Sync,
    Kind: SubKind + Sync,
    Transformer: ExchangeTransformer<Exchange, Kind>,
{
    async fn init(subscriptions: &[Subscription<Exchange, Kind>]) -> Result<Self, SocketError> {
        // Connect & subscribe
        let (
            websocket,
            map
        ) = Exchange::subscribe(&subscriptions);

        // Split WebSocket into WsStream & WsSink components
        let (_, ws_stream) = websocket.split();
        let (ws_sink_tx, _) = mpsc::unbounded_channel();

        // Construct Transformer associated with this Exchange and SubKind
        let transformer = Transformer::new(ws_sink_tx, map);

        Ok(ExchangeWsStream::new(ws_stream, transformer))
    }
}
























