use super::super::{
    BinanceServer,
    book::l2::{BinanceOrderBookL2Delta, BinanceOrderBookL2Snapshot},
};
use super::BinanceServerFuturesUsd;
use crate::{
    subscription::{Subscription, book::OrderBook},
    transformer::book::{InstrumentOrderBook, OrderBookUpdater},
};
use barter_integration::{
    error::SocketError, protocol::websocket::WsMessage,
};
use async_trait::async_trait;
use chrono::Utc;
use tokio::sync::mpsc;
use serde::{Deserialize, Serialize};

// Todo:
//  - Need a SocketError that causes the Stream to re-initialise at the `fn consume()` level
//  - OrderBook sorting when building the snapshot


/// Todo:
///
/// Notes:
///  - Receiving an event that removes a price level that is not in your local order book can happen and is normal.
///  - Uppercase U => first_update_id
///  - Lowercase u => last_update_id,
///
/// See docs: <https://binance-docs.github.io/apidocs/futures/en/#how-to-manage-a-local-order-book-correctly>
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Deserialize, Serialize)]
pub struct BinanceFuturesBookUpdater {
    updates_processed: u64,
    last_update_id: u64,
}

impl BinanceFuturesBookUpdater {
    /// Construct a new BinanceFutures [`OrderBookUpdater`] using the provided last_update_id from
    /// a HTTP snapshot.
    pub fn new(last_update_id: u64) -> Self {
        Self {
            updates_processed: 0,
            last_update_id
        }
    }

    /// BinanceFuturesUsd: How To Manage A Local OrderBook Correctly: Step 5:
    /// "The first processed event should have U <= lastUpdateId AND u >= lastUpdateId"
    ///
    /// See docs: <https://binance-docs.github.io/apidocs/futures/en/#how-to-manage-a-local-order-book-correctly>
    pub fn is_first_update(&self) -> bool {
        self.updates_processed == 0
    }

    /// BinanceFuturesUsd: How To Manage A Local OrderBook Correctly: Step 5:
    /// "The first processed event should have U <= lastUpdateId AND u >= lastUpdateId"
    ///
    /// See docs: <https://binance-docs.github.io/apidocs/futures/en/#how-to-manage-a-local-order-book-correctly>
    pub fn validate_first_update(&self, update: &BinanceOrderBookL2Delta) -> Result<(), SocketError> {
        if update.first_update_id > self.last_update_id {
            // Error
            todo!()
        }
        
        if update.last_update_id < self.last_update_id {
            // Error
            todo!()
        }
        
        Ok(())
    }

    /// BinanceFuturesUsd: How To Manage A Local OrderBook Correctly: Step 6:
    /// "While listening to the stream, each new event's pu should be equal to the previous
    ///  event's u, otherwise initialize the process from step 3."
    ///
    /// See docs: <https://binance-docs.github.io/apidocs/futures/en/#how-to-manage-a-local-order-book-correctly>
    pub fn validate_next_update(&self, update: &BinanceOrderBookL2Delta) -> Result<(), SocketError> {
        if update.prev_last_update_id != self.last_update_id {
            // Error
            todo!()
        }

        Ok(())
    }
}

#[async_trait]
impl OrderBookUpdater for BinanceFuturesBookUpdater {
    type OrderBook = OrderBook;
    type Update = BinanceOrderBookL2Delta;

    async fn init<Exchange, Kind>(
        _: mpsc::UnboundedSender<WsMessage>,
        subscription: Subscription<Exchange, Kind>
    ) -> Result<InstrumentOrderBook<Self>, SocketError>
    where
        Exchange: Send,
        Kind: Send,
    {
        // Construct initial OrderBook snapshot GET url
        let snapshot_url = format!(
            "{}?symbol={}{}&limit=100",
            BinanceServerFuturesUsd::http_book_snapshot_url(),
            subscription.instrument.base.as_ref().to_uppercase(),
            subscription.instrument.quote.as_ref().to_uppercase()
        );

        // Fetch initial OrderBook snapshot via HTTP
        let snapshot = reqwest::get(snapshot_url)
            .await?
            .json::<BinanceOrderBookL2Snapshot>()
            .await?;

        Ok(InstrumentOrderBook {
            instrument: subscription.instrument,
            updater: Self::new(snapshot.last_update_id),
            book: OrderBook::from(snapshot),
        })
    }

    fn update(&mut self, book: &mut Self::OrderBook, update: Self::Update) -> Result<Option<Self::OrderBook>, SocketError> {
        // BinanceFuturesUsd: How To Manage A Local OrderBook Correctly
        //
        // 1. Open a stream to wss://fstream.binance.com/stream?streams=<base><quote>@depth.
        // 2. Buffer the events you receive from the stream. For same price, latest received update
        //    covers the previous one.
        // 3. Get a depth snapshot from https://fapi.binance.com/fapi/v1/depth?symbol=BTCUSDT&limit=1000 .
        // 4. Drop any event where u is < lastUpdateId in the snapshot.
        // 5. The first processed event should have U <= lastUpdateId AND u >= lastUpdateId
        // 6. While listening to the stream, each new event's pu should be equal to the previous
        //    event's u, otherwise initialize the process from step 3.
        // 7. The data in each event is the absolute quantity for a price level.
        // 8. If the quantity is 0, remove the price level.
        //
        // Notes:
        //  - Receiving an event that removes a price level that is not in your local order book can happen and is normal.
        //  - Uppercase U => first_update_id
        //  - Lowercase u => last_update_id,
        //
        // See docs: <https://binance-docs.github.io/apidocs/futures/en/#how-to-manage-a-local-order-book-correctly>

        // 4. Drop any event where u is < lastUpdateId in the snapshot:
        if update.last_update_id < self.last_update_id {
            return Ok(None)
        }

        if self.is_first_update() {
            // 5. The first processed event should have U <= lastUpdateId AND u >= lastUpdateId
            self.validate_first_update(&update)?;
        }
        else {
            // 6. Each new event's pu should be equal to the previous event's u
            self.validate_next_update(&update)?;
        }

        // Update OrderBook metadata & Levels:
        // 7. The data in each event is the absolute quantity for a price level.
        // 8. If the quantity is 0, remove the price level.
        book.last_update_time = Utc::now();
        book.bids.upsert(update.bids);
        book.asks.upsert(update.asks);

        // Update OrderBookUpdater metadata
        self.updates_processed += 1;
        self.last_update_id = update.last_update_id;

        Ok(Some(book.snapshot()))
    }
}
