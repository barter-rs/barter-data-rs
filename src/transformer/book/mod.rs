use crate::{
    subscription::{book::OrderBook, Subscription}, error::DataError,
};
use barter_integration::{
    error::SocketError,
    model::{Instrument, SubscriptionId},
    protocol::websocket::WsMessage,
};
use std::{
    collections::HashMap,
};
use async_trait::async_trait;
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
        subscription: Subscription<Exchange, Kind>
    ) -> Result<InstrumentOrderBook<Self>, DataError>
    where
        Exchange: Send,
        Kind: Send;

    fn update(&mut self, book: &mut Self::OrderBook, update: Self::Update) -> Result<Option<Self::OrderBook>, DataError>;
}

// Todo:
pub struct OrderBookMap<Updater>(pub HashMap<SubscriptionId, InstrumentOrderBook<Updater>>);

impl<Updater> FromIterator<(SubscriptionId, InstrumentOrderBook<Updater>)> for OrderBookMap<Updater> {
    fn from_iter<Iter>(iter: Iter) -> Self
    where
        Iter: IntoIterator<Item = (SubscriptionId, InstrumentOrderBook<Updater>)>,
    {
        Self(iter.into_iter().collect::<HashMap<SubscriptionId, InstrumentOrderBook<Updater>>>())
    }
}

impl<Updater> OrderBookMap<Updater> {
    // Todo:
    pub fn find_book_mut(&mut self, id: &SubscriptionId) -> Result<&mut InstrumentOrderBook<Updater>, SocketError> {
        self.0
            .get_mut(id)
            .ok_or_else(|| SocketError::Unidentifiable(id.clone()))
    }
}

// Todo:
pub struct InstrumentOrderBook<Updater> {
    pub instrument: Instrument,
    pub updater: Updater,
    pub book: OrderBook,
}
