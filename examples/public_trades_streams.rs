use barter_data::{
    exchange::{ExchangeId, binance::futures::BinanceFuturesUsd},
    streams::Streams,
    subscription::trade::PublicTrades,
};
use barter_integration::model::InstrumentKind;
use tracing::info;

#[tokio::main]
async fn main() {
    // Initialise INFO Tracing log subscriber
    init_logging();

    // Initialise PublicTrades Streams for BinanceFuturesUsd only
    // '--> each call to StreamBuilder::subscribe() initialises a separate WebSocket connection
    let mut streams = Streams::builder()

        // Separate WebSocket connection for BTC_USDT stream since it's very high volume
        .subscribe([
            (BinanceFuturesUsd::default(), "btc", "usdt", InstrumentKind::FuturePerpetual, PublicTrades),
        ])

        // Separate WebSocket connection for ETH_USDT stream since it's very high volume
        .subscribe([
            (BinanceFuturesUsd::default(), "eth", "usdt", InstrumentKind::FuturePerpetual, PublicTrades),
        ])

        // Lower volume Instruments can share a WebSocket connection
        .subscribe([
            (BinanceFuturesUsd::default(), "xrp", "usdt", InstrumentKind::FuturePerpetual, PublicTrades),
            (BinanceFuturesUsd::default(), "sol", "usdt", InstrumentKind::FuturePerpetual, PublicTrades),
            (BinanceFuturesUsd::default(), "avax", "usdt", InstrumentKind::FuturePerpetual, PublicTrades),
            (BinanceFuturesUsd::default(), "ltc", "usdt", InstrumentKind::FuturePerpetual, PublicTrades),
        ])
        .init()
        .await
        .unwrap();

    // Select the ExchangeId::BinanceFuturesUsd stream
    let mut binance_stream = streams
        .select(ExchangeId::BinanceFuturesUsd)
        .unwrap();

    while let Some(trade) = binance_stream.recv().await {
        info!("Market<PublicTrade>: {trade:?}");
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