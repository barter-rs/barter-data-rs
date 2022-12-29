use std::collections::HashMap;
use tokio::sync::mpsc;
use barter_integration::error::SocketError;
use barter_integration::model::{Instrument, SubscriptionId};
use barter_integration::protocol::websocket::WsMessage;
use barter_integration::Transformer;
use crate::event::Market;
use crate::exchange::binance::{Binance, BinanceServer};
use crate::exchange::binance::book::l2::{BinanceOrderBookL2Delta, BinanceOrderBookL2Snapshot};
use crate::Identifier;
use crate::subscription::book::{OrderBook, OrderBooksL2};
use crate::subscription::{Subscription, SubscriptionMap};
use crate::transformer::ExchangeTransformer;
use async_trait::async_trait;

// Todo:
//  - should find_book / find_instrument take an Identifier<Option<&SubscriptionId>>?
//  - Configure depth, currently we have hard-coded to 100

// Todo: Bonus Points:
//  - Generalise this transformer if possible
//  - Impl FromIterator for SubscriptionMap
//  - Should SubscriptionMap actually just be InstrumentMap, or should InstrumentOB be SubscriptionOB?
//  - Use const generics w/ fixed sized arrays and Copy for Vec<Level>

/// Todo:
pub struct OrderBookMap(pub HashMap<SubscriptionId, InstrumentOrderBook>);

impl FromIterator<(SubscriptionId, InstrumentOrderBook)> for OrderBookMap {
    fn from_iter<Iter>(iter: Iter) -> Self
    where
        Iter: IntoIterator<Item = (SubscriptionId, InstrumentOrderBook)>,
    {
        Self(
            iter.into_iter()
                .collect::<HashMap<SubscriptionId, InstrumentOrderBook>>()
        )
    }
}

/// Todo:
pub struct InstrumentOrderBook {
    instrument: Instrument,
    book: OrderBook,
}

impl InstrumentOrderBook {
    /// Todo:
    pub fn new<I, OB>(instrument: I, book: OB) -> Self
    where
        I: Into<Instrument>,
        OB: Into<OrderBook>,
    {
        Self {
            instrument: instrument.into(),
            book: book.into()
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

#[async_trait]
impl<Server> ExchangeTransformer<Binance<Server>, OrderBooksL2> for BookL2DeltaTransformer
where
    Server: BinanceServer + 'static,
{
    async fn new(_: mpsc::UnboundedSender<WsMessage>, map: SubscriptionMap<Binance<Server>, OrderBooksL2>) -> Result<Self, SocketError>
    {
        // Fetch initial OrderBook snapshots for all Subscriptions via HTTP
        let futures = map
            .0
            .into_iter()
            .map(|(sub_id, subscription)| {
                fetch_initial_order_book(sub_id, subscription)
            });

        // Await all initial OrderBook snapshot requests
        let subscription_order_books = futures::future::join_all(futures).await;

        // Construct OrderBookMap if all requests successful
        let books = subscription_order_books
            .into_iter()
            .collect::<Result<OrderBookMap, SocketError>>()?;

        Ok(Self {
            books
        })
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

async fn fetch_initial_order_book<Server>(
    subscription_id: SubscriptionId,
    subscription: Subscription<Binance<Server>, OrderBooksL2>,
) -> Result<(SubscriptionId, InstrumentOrderBook), SocketError>
where
    Server: BinanceServer,
{
    // Construct initial OrderBook snapshot GET url
    let snapshot_url = format!(
        "{}?symbol={}{}&limit=100",
        Server::http_book_snapshot_url(),
        subscription.instrument.base.as_ref().to_uppercase(),
        subscription.instrument.quote.as_ref().to_uppercase()
    );

    // Fetch initial OrderBook snapshot via HTTP
    let snapshot = reqwest::get(snapshot_url)
        .await?
        .json::<BinanceOrderBookL2Snapshot>()
        .await
        .map(OrderBook::from)?;

    Ok((subscription_id, InstrumentOrderBook::new(subscription.instrument, snapshot)))
}