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

    // Initialise PublicTrades Streams for various exchanges
    // '--> each call to StreamBuilder::subscribe() initialises a separate WebSocket connection
    let trade_streams = Streams::builder()
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
        .init()
        .await
        .unwrap();

    // Initialise OrderBooksL1 Stream for various exchanges
    // '--> each call to StreamBuilder::subscribe() initialises a separate WebSocket connection
    let book_l1_streams = Streams::builder()
        .subscribe([
            (BinanceSpot::default(), "btc", "usdt", InstrumentKind::Spot, OrderBooksL1),
        ])
        .subscribe([
            (BinanceFuturesUsd::default(), "btc", "usdt", InstrumentKind::FuturePerpetual, OrderBooksL1),
        ])
        .init()
        .await
        .unwrap();

    // Initialise OrderBooksL2 Stream for various exchanges
    // '--> each call to StreamBuilder::subscribe() initialises a separate WebSocket connection
    let book_l2_streams = Streams::builder()
        .subscribe([
            (BinanceSpot::default(), "btc", "usdt", InstrumentKind::Spot, OrderBooksL2),
        ])
        .subscribe([
            (BinanceFuturesUsd::default(), "btc", "usdt", InstrumentKind::FuturePerpetual, OrderBooksL2),
        ])
        .init()
        .await
        .unwrap();

    // Todo:
    let mut merged_stream: mpsc::UnboundedReceiver<MarketEvent<DataKind>> = trade_streams
        .merge(book_l1_streams)
        .await
        .merge(book_l2_streams)
        .await
        .merged_rx;

    while let Some(event) = merged_stream.recv().await {
        info!("MarketEvent<DataKind>: {event:?}");
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
