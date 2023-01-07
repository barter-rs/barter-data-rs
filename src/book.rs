
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
use serde::Deserialize;

// Todo:
pub trait OrderBookManager {
    type Update: Identifier<Option<SubscriptionId>>;
    type Output;

    fn update(&mut self, update: Self::Update) -> Result<(), SocketError>;
    fn snapshot(&self) -> Result<Self::Output, SocketError>;
}

// Todo:
pub struct OrderBookMap<OB>(pub HashMap<SubscriptionId, InstrumentOrderBook<OB>>);

// Todo:
pub struct InstrumentOrderBook<OB> {
    instrument: Instrument,
    book: OB,
}

// Todo:
impl<OB> OrderBookMap<OB> {
    pub fn find_book_mut(&mut self, id: &SubscriptionId) -> Result<&mut InstrumentOrderBook<OB>, SocketError> {
        self.0
            .get_mut(id)
            .ok_or_else(|| SocketError::Unidentifiable(id.clone()))
    }
}

// Todo:
pub struct BookTransformer<Exchange, OB> {
    pub books: OrderBookMap<OB>,
    phantom: PhantomData<Exchange>,
}

impl<Exchange, OB> Transformer for BookTransformer<Exchange, OB>
where
    Exchange: Connector,
    OB: OrderBookManager<Output = OrderBook>,
    OB::Update: for<'de> Deserialize<'de>,
{
    type Input = OB::Update;
    type Output = Market<OrderBook>;
    type OutputIter = Vec<Result<Self::Output, SocketError>>;

    fn transform(&mut self, update: Self::Input) -> Self::OutputIter {
        // Determine if the update has an identifiable SubscriptionId
        let subscription_id = match update.id() {
            Some(subscription_id) => subscription_id,
            None => return vec![],
        };

        // Retrieve the OrderBookManager associated with this update (snapshot or delta)
        let book = match self.books.find_book_mut(&subscription_id) {
            Ok(book) => book,
            Err(unidentifiable) => return vec![Err(unidentifiable)],
        };

        // De-structure for ease
        let InstrumentOrderBook { instrument, book } = book;

        // Apply update (snapshot or delta) to OrderBookManager
        if let Err(error) = book.update(update) {
            return vec![Err(error)]
        }

        // Generate normalised Barter OrderBook snapshot
        match book.snapshot() {
            Ok(snapshot) => MarketIter::<OrderBook>::from((Exchange::ID, instrument.clone(), snapshot)).0,
            Err(error) => vec![Err(error)]
        }
    }
}
