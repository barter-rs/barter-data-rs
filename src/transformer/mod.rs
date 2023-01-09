use crate::{
    error::DataError,
    event::Market,
    subscription::{SubKind, SubscriptionMap},
};
use async_trait::async_trait;
use barter_integration::{protocol::websocket::WsMessage, Transformer};
use tokio::sync::mpsc;

/// Todo:
/// Generic OrderBook [`ExchangeTransformer`] implementations.
pub mod book;

/// Generic stateless [`ExchangeTransformer`] implementation - often used for transforming
/// [`PublicTrade`](crate::subscription::trade::PublicTrade) streams.
pub mod stateless;

/// Todo:
#[async_trait]
pub trait ExchangeTransformer<Exchange, Kind>
where
    Self: Transformer<Output = Market<Kind::Event>, Error = DataError> + Sized,
    Kind: SubKind,
{
    /// Todo:
    async fn new(
        ws_sink_tx: mpsc::UnboundedSender<WsMessage>,
        map: SubscriptionMap<Exchange, Kind>,
    ) -> Result<Self, DataError>;
}
