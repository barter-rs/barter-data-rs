use crate::{error::DataError, subscription::book::OrderBook};
use async_trait::async_trait;
use barter_integration::{model::Instrument, protocol::websocket::WsMessage};
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;

/// Todo:
pub mod multi;

/// Todo:
#[async_trait]
pub trait OrderBookUpdater
where
    Self: Sized,
{
    type OrderBook;
    type Update;

    async fn init<Exchange, Kind>(
        ws_sink_tx: mpsc::UnboundedSender<WsMessage>,
        instrument: Instrument,
    ) -> Result<InstrumentOrderBook<Self>, DataError>
    where
        Exchange: Send,
        Kind: Send;

    fn update(
        &mut self,
        book: &mut Self::OrderBook,
        update: Self::Update,
    ) -> Result<Option<Self::OrderBook>, DataError>;
}

/// Todo:
#[derive(Clone, PartialEq, PartialOrd, Debug, Deserialize, Serialize)]
pub struct InstrumentOrderBook<Updater> {
    pub instrument: Instrument,
    pub updater: Updater,
    pub book: OrderBook,
}
