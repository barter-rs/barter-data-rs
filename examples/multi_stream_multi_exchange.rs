use barter_data::{
    exchange::{
        binance::{futures::BinanceFuturesUsd, spot::BinanceSpot},
        okx::Okx,
    },
    streams::Streams,
    event::MarketEvent,
    subscription::{
        trade::{PublicTrades, PublicTrade},
        book::{OrderBooksL1, OrderBookL1},
    },
};
use barter_integration::model::InstrumentKind;
use tokio::sync::mpsc;
use tokio_stream::StreamExt;
use tracing::info;
use barter_data::streams::builder::multi::MultiStreamBuilder;
use barter_data::subscription::book::{OrderBook, OrderBooksL2};

// Define custom enum to hold multiple market event kinds
#[derive(Debug)]
pub enum DataKind {
    Trade(PublicTrade),
    OrderBookL1(OrderBookL1),
    OrderBook(OrderBook),
}

// Provide From mappings for each variant to satisfy Streams::merge() trait bounds
impl From<PublicTrade> for DataKind {
    fn from(trade: PublicTrade) -> Self {
        Self::Trade(trade)
    }
}
impl From<OrderBookL1> for DataKind {
    fn from(book_l1: OrderBookL1) -> Self {
        Self::OrderBookL1(book_l1)
    }
}
impl From<OrderBook> for DataKind {
    fn from(book: OrderBook) -> Self {
        Self::OrderBook(book)
    }
}

#[rustfmt::skip]
#[tokio::main]
async fn main() {
    // Initialise INFO Tracing log subscriber
    init_logging();

    // Note:
    // - Each call to StreamBuilder::subscribe() creates a separate WebSocket connection for those
    //   Subscriptions passed.

    // Initialise MarketEvent<DataKind> Streams for various exchanges
    let streams: Streams<MarketEvent<DataKind>> = MultiStreamBuilder::new()
        // Add PublicTrades Streams for various exchanges
        .add(Streams::<PublicTrades>::builder()
            .subscribe([
                (BinanceSpot::default(), "btc", "usdt", InstrumentKind::Spot, PublicTrades),
            ])
            .subscribe([
                (BinanceFuturesUsd::default(), "btc", "usdt", InstrumentKind::FuturePerpetual, PublicTrades),
            ])
            .subscribe([
                (Okx, "btc", "usdt", InstrumentKind::Spot, PublicTrades),
                (Okx, "btc", "usdt", InstrumentKind::FuturePerpetual, PublicTrades),
            ])
        )

        // Add OrderBooksL1 Stream for various exchanges
        .add(Streams::<OrderBooksL1>::builder()
            .subscribe([
                (BinanceSpot::default(), "btc", "usdt", InstrumentKind::Spot, OrderBooksL1),
            ])
            .subscribe([
                (BinanceFuturesUsd::default(), "btc", "usdt", InstrumentKind::FuturePerpetual, OrderBooksL1),
            ])
        )

        // Add OrderBooksL2 Stream for various exchanges
        .add(Streams::<OrderBooksL2>::builder()
            .subscribe([
                (BinanceSpot::default(), "btc", "usdt", InstrumentKind::Spot, OrderBooksL2),
            ])
            .subscribe([
                (BinanceFuturesUsd::default(), "btc", "usdt", InstrumentKind::FuturePerpetual, OrderBooksL2),
            ])
        )
        .init()
        .await
        .unwrap();

    // Join all exchange Streams into a single tokio_stream::StreamMap
    // Notes:
    //  - Use `streams.select(ExchangeId)` to interact with the individual exchange streams!
    //  - Use `streams.join()` to join all exchange streams into a single mpsc::UnboundedReceiver!
    let mut joined_stream = streams.join_map().await;

    while let Some((exchange, data)) = joined_stream.next().await {
        info!("Exchange: {exchange}, MarketEvent<DataKind>: {data:?}");
    }
}

// Initialise an INFO `Subscriber` for `Tracing` Json logs and install it as the global default.
fn init_logging() {
    tracing_subscriber::fmt()
        // Filter messages based on the INFO
        .with_env_filter(
            tracing_subscriber::filter::EnvFilter::builder()
                .with_default_directive(tracing_subscriber::filter::LevelFilter::INFO.into())
                .from_env_lossy(),
        )
        // Disable colours on release builds
        .with_ansi(cfg!(debug_assertions))
        // Enable Json formatting
        .json()
        // Install this Tracing subscriber as global default
        .init()
}


