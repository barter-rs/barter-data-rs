use std::collections::HashMap;
use tokio::sync::mpsc;
use barter_integration::error::SocketError;
use barter_integration::model::{Instrument, SubscriptionId};
use barter_integration::protocol::websocket::WsMessage;
use barter_integration::Transformer;
use crate::event::Market;
use crate::exchange::binance::{Binance, BinanceServer};
use crate::exchange::binance::book::l2::BinanceOrderBookL2Delta;
use crate::Identifier;
use crate::subscription::book::{Level, OrderBook, OrderBooksL2};
use crate::subscription::{Subscription, SubscriptionMap};
use crate::transformer::ExchangeTransformer;
use async_trait::async_trait;

// Todo:
//  - Create new type for BookMap<SubscriptionId, OrderBook>, Or it could be &str, OrderBook?
//  - should find_book / find_instrument take an Identifier<Option<&SubscriptionId>>?
//  - Configure depth, currently we have hard-coded to 50

// Todo: Bonus Points:
//  - Generalise this transformer if possible

pub struct OrderBookMap(pub HashMap<SubscriptionId, InstrumentOrderBook>);

pub struct InstrumentOrderBook {
    instrument: Instrument,
    book: OrderBook,
}

impl InstrumentOrderBook {
    pub fn new(instrument: Instrument, book: OrderBook) -> Self {
        Self {
            instrument,
            book
        }
    }
}

impl<Exchange, Kind> From<SubscriptionMap<Exchange, Kind>> for OrderBookMap {
    fn from(sub_map: SubscriptionMap<Exchange, Kind>) -> Self {
        Self(sub_map
            .0
            .into_iter()
            .map(|(sub_id, Subscription { instrument, .. } )| {
                (sub_id, InstrumentOrderBook::new(instrument, OrderBook::default()))
            })
            .collect()
        )
    }
}

impl OrderBookMap {
    pub fn find_book_mut(&mut self, id: &SubscriptionId) -> Result<&mut InstrumentOrderBook, SocketError> {
        self.0
            .get_mut(id)
            .ok_or_else(|| SocketError::Unidentifiable(id.clone()))
    }
}

pub(super) struct BookL2DeltaTransformer {
    pub books: OrderBookMap,
}

pub(super) struct BinanceOrderBookSnapshot {
    last_update_id: u64,
    bids: Vec<Level>,
    asks: Vec<Level>,
}

#[async_trait]
impl<Server> ExchangeTransformer<Binance<Server>, OrderBooksL2> for BookL2DeltaTransformer
where
    Server: BinanceServer + 'static,
{
    async fn new(_: mpsc::UnboundedSender<WsMessage>, map: SubscriptionMap<Binance<Server>, OrderBooksL2>) -> Result<Self, SocketError>
    {
        struct Lego {
            subscription_id: SubscriptionId,
            instrument: Instrument,
            snapshot_url: String,
        }

        // Get initial snapshots urls for all the Subscription OrderBooks
        let book_legos = map
            .0
            .into_iter()
            .map(|(sub_id, sub)| {
                Lego {
                    subscription_id: sub_id,
                    snapshot_url:format!(
                        "{}?symbol={}{}&limit=50",
                        Server::http_book_snapshot_url(),
                        sub.instrument.base.as_ref().to_uppercase(),
                        sub.instrument.quote.as_ref().to_uppercase()
                    ),
                    instrument: sub.instrument,
                }
            })
            .collect::<Vec<Lego>>();

        // Todo: Send requests in parallel... where did I do that recently? I can't remember...
        // Ahh it was somewhere in the simulated exchange / exchange client

        // Self {
        //     books: OrderBookMap::from(map),
        // }

        todo!()
    }
}

impl Transformer for BookL2DeltaTransformer {
    type Input = BinanceOrderBookL2Delta;
    type Output = Market<OrderBook>;
    type OutputIter = Vec<Result<Self::Output, SocketError>>;

    fn transform(&mut self, delta: Self::Input) -> Self::OutputIter {
        // Determine if the message has an identifiable SubscriptionId
        let subscription_id = match delta.id() {
            Some(subscription_id) => subscription_id,
            None => return vec![],
        };

        // Retrieve the OrderBook associated with this delta
        let book = match self.books.find_book_mut(&subscription_id) {
            Ok(book) => book,
            Err(unidentifiable) => return vec![Err(unidentifiable)],
        };

        // De-structure for ease
        let InstrumentOrderBook { instrument, book } = book;

        todo!()
    }
}