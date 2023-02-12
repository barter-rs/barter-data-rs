use barter_integration::model::InstrumentKind;
use barter_data::exchange::ExchangeId;
use barter_data::exchange::bybit::spot::BybitSpot;
use barter_data::streams::Streams;
use barter_data::subscription::trade::PublicTrades;
use tracing::info;

#[rustfmt::skip]
#[tokio::main]
async fn main() {
    init_logging();

    let mut streams = Streams::<PublicTrades>::builder()
        .subscribe([
            (BybitSpot::default(), "btc", "usdt", InstrumentKind::Spot, PublicTrades),
        ]).init()
        .await
        .unwrap();

    let mut bybit_stream = streams
        .select(ExchangeId::BybitSpot)
        .unwrap();

    while let Some(trade) = bybit_stream.recv().await {
        info!("MarketEvent<PublicTrade>: {trade:?}");
    }

}

fn init_logging() {
    tracing_subscriber::fmt()
        // Filter messages based on the INFO
        .with_env_filter(
            tracing_subscriber::filter::EnvFilter::builder()
                .with_default_directive(tracing_subscriber::filter::LevelFilter::INFO.into())
                .from_env_lossy(),
        )
        .with_ansi(cfg!(debug_assertions))
        .json()
        .init()
}