
use barter_integration::{
    error::SocketError,
    model::{Instrument, SubscriptionId},
    Transformer,
};
use crate::{
    event::{Market, MarketIter},
    exchange::Connector,
    subscription::book::OrderBook,
    Identifier,
};
use std::{
    collections::HashMap,
    marker::PhantomData,
};
use async_trait::async_trait;
use serde::Deserialize;
use tokio::sync::mpsc;
use barter_integration::protocol::websocket::WsMessage;
use crate::subscription::{SubKind, Subscription, SubscriptionMap};
use crate::transformer::ExchangeTransformer;

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
    ) -> Result<InstrumentOrderBook<Self>, SocketError>
    where
        Exchange: Send,
        Kind: Send;

    fn update(&mut self, book: &mut Self::OrderBook, update: Self::Update) -> Result<(), SocketError>;
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

// Todo:
pub struct BookTransformer<Exchange, Kind, Updater> {
    pub book_map: OrderBookMap<Updater>,
    phantom: PhantomData<(Exchange, Kind)>,
}

#[async_trait]
impl<Exchange, Kind, Updater> ExchangeTransformer<Exchange, Kind> for BookTransformer<Exchange, Kind, Updater>
where
    Exchange: Connector + Send,
    Kind: SubKind<Event = OrderBook> + Send,
    Updater: OrderBookUpdater<OrderBook = Kind::Event> + Send,
    Updater::Update: Identifier<Option<SubscriptionId>> + for<'de> Deserialize<'de>,
{
    async fn new(
        ws_sink_tx: mpsc::UnboundedSender<WsMessage>,
        map: SubscriptionMap<Exchange, Kind>
    ) -> Result<Self, SocketError>
    {
        // Initialise InstrumentOrderBooks for all Subscriptions
        let (sub_ids, init_book_requests): (Vec<_>, Vec<_>)  = map
            .0
            .into_iter()
            .map(|(sub_id, subscription)| {
                (sub_id, Updater::init(ws_sink_tx.clone(), subscription))
            })
            .unzip();

        // Await all initial OrderBook snapshot requests
        let init_order_books = futures::future::join_all(init_book_requests)
            .await
            .into_iter()
            .collect::<Result<Vec<InstrumentOrderBook<Updater>>, SocketError>>()?;

        // Construct OrderBookMap if all requests successful
        let book_map = sub_ids
            .into_iter()
            .zip(init_order_books.into_iter())
            .collect::<OrderBookMap<Updater>>();

        Ok(Self { book_map, phantom: PhantomData::default() })
    }
}

impl<Exchange, Kind, Updater> Transformer for BookTransformer<Exchange, Kind, Updater>
where
    Exchange: Connector,
    Kind: SubKind<Event = OrderBook>,
    Updater: OrderBookUpdater<OrderBook = Kind::Event>,
    Updater::Update: Identifier<Option<SubscriptionId>> + for<'de> Deserialize<'de>,
{
    type Input = Updater::Update;
    type Output = Market<Kind::Event>;
    type OutputIter = Vec<Result<Self::Output, SocketError>>;

    fn transform(&mut self, update: Self::Input) -> Self::OutputIter {
        // Determine if the update has an identifiable SubscriptionId
        let subscription_id = match update.id() {
            Some(subscription_id) => subscription_id,
            None => return vec![],
        };

        // Retrieve the InstrumentOrderBook associated with this update (snapshot or delta)
        let book = match self.book_map.find_book_mut(&subscription_id) {
            Ok(book) => book,
            Err(unidentifiable) => return vec![Err(unidentifiable)],
        };

        // De-structure for ease
        let InstrumentOrderBook {
            instrument, book, updater,
        } = book;

        // Apply update (snapshot or delta) to OrderBook & generate Market<OrderBook> snapshot
        match updater.update(book, update) {
            Ok(()) => MarketIter::<OrderBook>::from((Exchange::ID, instrument.clone(), book.clone())).0,
            Err(error) => vec![Err(error)]
        }
    }
}
