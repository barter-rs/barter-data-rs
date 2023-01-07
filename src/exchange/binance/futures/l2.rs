use async_trait::async_trait;
use tokio::sync::mpsc;
use barter_integration::error::SocketError;
use barter_integration::protocol::websocket::WsMessage;
use crate::book::{InstrumentOrderBook, OrderBookUpdater};
use crate::exchange::binance::BinanceServer;
use crate::exchange::binance::book::l2::BinanceOrderBookL2Snapshot;
use crate::exchange::binance::futures::BinanceServerFuturesUsd;
use crate::subscription::book::OrderBook;
use crate::subscription::Subscription;

pub struct BinanceFuturesBookUpdater {
    first_update_id: u64,
    last_update_id: u64,
}

pub struct BinanceFuturesOrderBookL2Delta;

#[async_trait]
impl OrderBookUpdater for BinanceFuturesBookUpdater {
    type OrderBook = OrderBook;
    type Update = BinanceFuturesOrderBookL2Delta;

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
            .await
            .map(OrderBook::from)?;

        Ok(InstrumentOrderBook {
            instrument: subscription.instrument,
            updater: Self { first_update_id: 0, last_update_id: snapshot.last_update_id },
            book: snapshot,
        })
    }

    fn update(&mut self, book: &mut Self::OrderBook, update: Self::Update) -> Result<(), SocketError> {
        todo!()
    }
}
