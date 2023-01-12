use barter_data::{
    exchange::{ExchangeId, binance::spot::BinanceSpot},
    streams::Streams,
    subscription::book::OrderBooksL1,
};
use barter_integration::model::InstrumentKind;
use tracing::info;

#[rustfmt::skip]
#[tokio::main]
async fn main() {
    // Initialise INFO Tracing log subscriber
    init_logging();

    // Initialise OrderBooksL1 Streams for BinanceSpot only
    // '--> each call to StreamBuilder::subscribe() initialises a separate WebSocket connection
    let mut streams = Streams::builder()

        // Separate WebSocket connection for BTC_USDT stream since it's very high volume
        .subscribe([
            (BinanceSpot::default(), "btc", "usdt", InstrumentKind::Spot, OrderBooksL1),
        ])

        // Separate WebSocket connection for ETH_USDT stream since it's very high volume
        .subscribe([
            (BinanceSpot::default(), "eth", "usdt", InstrumentKind::Spot, OrderBooksL1),
        ])

        // Lower volume Instruments can share a WebSocket connection
        .subscribe([
            (BinanceSpot::default(), "xrp", "usdt", InstrumentKind::Spot, OrderBooksL1),
            (BinanceSpot::default(), "sol", "usdt", InstrumentKind::Spot, OrderBooksL1),
            (BinanceSpot::default(), "avax", "usdt", InstrumentKind::Spot, OrderBooksL1),
            (BinanceSpot::default(), "ltc", "usdt", InstrumentKind::Spot, OrderBooksL1),
        ])
        .init()
        .await
        .unwrap();

    // Select the ExchangeId::BinanceSpot stream
    let mut binance_stream = streams
        .select(ExchangeId::BinanceSpot)
        .unwrap();

    while let Some(order_book_l1) = binance_stream.recv().await {
        info!("Market<OrderBookL1>: {order_book_l1:?}");
    }
}

// Initialise an INFO `Subscriber` for `Tracing` Json logs and install it as the global default.
fn init_logging() {
    tracing_subscriber::fmt()
        // Filter messages based on the INFO
        .with_env_filter(tracing_subscriber::filter::EnvFilter::builder()
            .with_default_directive(tracing_subscriber::filter::LevelFilter::INFO.into())
            .from_env_lossy()
        )
        // Disable colours on release builds
        .with_ansi(cfg!(debug_assertions))
        // Enable Json formatting
        .json()
        // Install this Tracing subscriber as global default
        .init()
}