use crate::{error::DataError, subscription::book::OrderBook};
use async_trait::async_trait;
use barter_integration::{
    error::SocketError,
    model::{Instrument, SubscriptionId},
    protocol::websocket::WsMessage,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tokio::sync::mpsc;

/// Todo:
pub mod multi;

// Todo:
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

/// Convenient type alias for a `HashMap` containing the mapping between a [`SubscriptionId`] and
/// the associated Barter [`InstrumentOrderBook`].
///
/// Used by [`OrderBook`] related [`ExchangeTransformers`](crate::transformer::ExchangeTransformer)
/// to identify the Barter [`InstrumentOrderBook`] associated with incoming exchange messages.
#[derive(Clone, PartialEq, Debug, Deserialize, Serialize)]
pub struct OrderBookMap<Updater>(pub HashMap<SubscriptionId, InstrumentOrderBook<Updater>>);

impl<Updater> FromIterator<(SubscriptionId, InstrumentOrderBook<Updater>)>
    for OrderBookMap<Updater>
{
    fn from_iter<Iter>(iter: Iter) -> Self
    where
        Iter: IntoIterator<Item = (SubscriptionId, InstrumentOrderBook<Updater>)>,
    {
        Self(
            iter.into_iter()
                .collect::<HashMap<SubscriptionId, InstrumentOrderBook<Updater>>>(),
        )
    }
}

impl<Updater> OrderBookMap<Updater> {
    /// Find the [`InstrumentOrderBook`] associated with the provided [`SubscriptionId`] reference.
    pub fn find_book_mut(
        &mut self,
        id: &SubscriptionId,
    ) -> Result<&mut InstrumentOrderBook<Updater>, SocketError> {
        self.0
            .get_mut(id)
            .ok_or_else(|| SocketError::Unidentifiable(id.clone()))
    }
}

// Todo:
#[derive(Clone, PartialEq, PartialOrd, Debug, Deserialize, Serialize)]
pub struct InstrumentOrderBook<Updater> {
    pub instrument: Instrument,
    pub updater: Updater,
    pub book: OrderBook,
}
