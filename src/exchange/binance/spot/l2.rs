use super::super::{
    BinanceServer,
    book::{
        BinanceLevel,
        l2::BinanceOrderBookL2Snapshot
    },
};
use super::BinanceServerSpot;
use crate::{Identifier, subscription::{book::OrderBook, Subscription}, transformer::book::{InstrumentOrderBook, OrderBookUpdater}};
use barter_integration::{
    error::SocketError, protocol::websocket::WsMessage, model::SubscriptionId,
};
use async_trait::async_trait;
use chrono::Utc;
use tokio::sync::mpsc;
use serde::{Deserialize, Serialize};

/// [`BinanceSpot`] OrderBook Level2 deltas WebSocket message.
///
/// See docs: <https://binance-docs.github.io/apidocs/spot/en/#partial-book-depth-streams>
#[derive(Clone, PartialEq, PartialOrd, Debug, Deserialize, Serialize)]
pub struct BinanceSpotOrderBookL2Delta {
    #[serde(alias = "s", deserialize_with = "super::super::book::l2::de_ob_l2_subscription_id")]
    pub subscription_id: SubscriptionId,

    #[serde(alias = "U")]
    pub first_update_id: u64,

    #[serde(alias = "u")]
    pub last_update_id: u64,

    #[serde(alias = "b")]
    pub bids: Vec<BinanceLevel>,

    #[serde(alias = "a")]
    pub asks: Vec<BinanceLevel>,
}

impl Identifier<Option<SubscriptionId>> for BinanceSpotOrderBookL2Delta {
    fn id(&self) -> Option<SubscriptionId> {
        Some(self.subscription_id.clone())
    }
}


/// [`Binance`] [`BinanceServerSpot`] [`OrderBookUpdater`].
///
/// BinanceSpot: How To Manage A Local OrderBook Correctly
///
/// 1. Open a stream to wss://stream.binance.com:9443/ws/<base><quote>@depth.
/// 2. Buffer the events you receive from the stream.
/// 3. Get a depth snapshot from https://api.binance.com/api/v3/depth?symbol=BNBBTC&limit=1000.
/// 4. -- *DIFFERENT FROM FUTURES* --
///    Drop any event where u is <= lastUpdateId in the snapshot.
/// 5. -- *DIFFERENT FROM FUTURES* --
///    The first processed event should have U <= lastUpdateId+1 AND u >= lastUpdateId+1.
/// 6. -- *DIFFERENT FROM FUTURES* --
///    While listening to the stream, each new event's U should be equal to the
///    previous event's u+1, otherwise initialize the process from step 3.
/// 7. The data in each event is the absolute quantity for a price level.
/// 8. If the quantity is 0, remove the price level.
///
/// Notes:
///  - Receiving an event that removes a price level that is not in your local order book can happen and is normal.
///  - Uppercase U => first_update_id
///  - Lowercase u => last_update_id,
///
/// See docs: <https://binance-docs.github.io/apidocs/spot/en/#how-to-manage-a-local-order-book-correctly>
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Deserialize, Serialize)]
pub struct BinanceSpotBookUpdater {
    updates_processed: u64,
    last_update_id: u64,
    prev_last_update_id: u64,
}

impl BinanceSpotBookUpdater {
    /// Construct a new BinanceSpot [`OrderBookUpdater`] using the provided last_update_id from
    /// a HTTP snapshot.
    pub fn new(last_update_id: u64) -> Self {
        Self {
            updates_processed: 0,
            prev_last_update_id: last_update_id,
            last_update_id,
        }
    }

    /// BinanceSpot: How To Manage A Local OrderBook Correctly: Step 5:
    /// "The first processed event should have U <= lastUpdateId+1 AND u >= lastUpdateId+1"
    ///
    /// See docs: <https://binance-docs.github.io/apidocs/spot/en/#how-to-manage-a-local-order-book-correctly>
    pub fn is_first_update(&self) -> bool {
        self.updates_processed == 0
    }

    /// BinanceSpot: How To Manage A Local OrderBook Correctly: Step 5:
    /// "The first processed event should have U <= lastUpdateId+1 AND u >= lastUpdateId+1"
    ///
    /// See docs: <https://binance-docs.github.io/apidocs/spot/en/#how-to-manage-a-local-order-book-correctly>
    pub fn validate_first_update(&self, update: &BinanceSpotOrderBookL2Delta) -> Result<(), SocketError> {
        if update.first_update_id > self.last_update_id + 1 {
            // Error
            todo!()
        }

        if update.last_update_id < self.last_update_id + 1 {
            // Error
            todo!()
        }

        Ok(())
    }

    /// BinanceFuturesUsd: How To Manage A Local OrderBook Correctly: Step 6:
    /// "While listening to the stream, each new event's U should be equal to the
    ///  previous event's u+1, otherwise initialize the process from step 3."
    ///
    /// See docs: <https://binance-docs.github.io/apidocs/spot/en/#how-to-manage-a-local-order-book-correctly>
    pub fn validate_next_update(&self, update: &BinanceSpotOrderBookL2Delta) -> Result<(), SocketError> {
        if update.first_update_id != self.prev_last_update_id + 1 {
            // Error
            todo!()
        }

        Ok(())
    }
}

#[async_trait]
impl OrderBookUpdater for BinanceSpotBookUpdater {
    type OrderBook = OrderBook;
    type Update = BinanceSpotOrderBookL2Delta;

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
            BinanceServerSpot::http_book_snapshot_url(),
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
        // BinanceSpot: How To Manage A Local OrderBook Correctly
        // See Self's Rust Docs for more information on each numbered step
        // See docs: <https://binance-docs.github.io/apidocs/spot/en/#how-to-manage-a-local-order-book-correctly>

        // 4. Drop any event where u is <= lastUpdateId in the snapshot:
        if update.last_update_id < self.last_update_id {
            return Ok(None)
        }

        if self.is_first_update() {
            // 5. The first processed event should have U <= lastUpdateId AND u >= lastUpdateId:
            self.validate_first_update(&update)?;
        }
        else {
            // 6. Each new event's pu should be equal to the previous event's u:
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
        self.prev_last_update_id = self.last_update_id;
        self.last_update_id = update.last_update_id;

        Ok(Some(book.snapshot()))
    }
}