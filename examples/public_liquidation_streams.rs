use barter_data::{
    exchange::{binance::futures::BinanceFuturesUsd, bybit::futures::BybitFuturesUsd, ExchangeId},
    streams::Streams,
    subscription::liquidation::Liquidations,
};
use barter_integration::model::InstrumentKind;
use futures::StreamExt;
use tracing::info;

#[rustfmt::skip]
#[tokio::main]
async fn main() {
    // Initialise INFO Tracing log subscriber
    init_logging();

    // Initialise Liquidations Streams
    // '--> each call to StreamBuilder::subscribe() creates a separate WebSocket connection
    let mut streams = Streams::<Liquidations>::builder()
        .subscribe([
            (BinanceFuturesUsd::default(), "eth", "usdt", InstrumentKind::FuturePerpetual, Liquidations),
        ])
        .subscribe([
            (BybitFuturesUsd::default(), "btc", "usdt", InstrumentKind::FuturePerpetual, Liquidations),
        ])
        .init()
        .await
        .unwrap();

    // Select the ExchangeId::BinanceFuturesUsd stream
    // Notes:
    //  - Use `streams.select(ExchangeId)` to interact with the individual exchange streams!
    //  - Use `streams.join()` to join all exchange streams into a single mpsc::UnboundedReceiver!
    let mut joined_stream = streams.join_map().await;

    while let Some((exchange, liquidation)) = joined_stream.next().await {
        info!("Exchange: {exchange}, MarketEvent<Liquidations>: {liquidation:?}");
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
