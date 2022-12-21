use crate::{
    event::Market,
    subscription::{SubKind, SubscriptionMap},
};
use barter_integration::{protocol::websocket::WsMessage, Transformer};
use tokio::sync::mpsc;
use barter_integration::error::SocketError;

/// Generic OrderBook [`ExchangeTransformer`] implementations.
pub mod book;

/// Generic stateless [`ExchangeTransformer`] implementation - often used for transforming
/// [`PublicTrade`](crate::subscription::trade::PublicTrade) streams.
pub mod stateless;

/// Todo:
pub trait ExchangeTransformer<Exchange, Kind>
where
    Self: Transformer<Output = Market<Kind::Event>>,
    Kind: SubKind,
{
    /// Todo:
    fn new(
        ws_sink_tx: mpsc::UnboundedSender<WsMessage>,
        map: SubscriptionMap<Exchange, Kind>,
    ) -> Result<Self, SocketError>;
}
