use crate::{
    error::DataError,
    event::Market,
    subscription::{Map, SubKind},
};
use async_trait::async_trait;
use barter_integration::{model::Instrument, protocol::websocket::WsMessage, Transformer};
use tokio::sync::mpsc;

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
    /// Construct a new [`Self`].
    ///
    /// The [`mpsc::UnboundedSender`] can be used by [`Self`] to send messages back to the exchange.
    async fn new(
        ws_sink_tx: mpsc::UnboundedSender<WsMessage>,
        instrument_map: Map<Instrument>,
    ) -> Result<Self, DataError>;
}
