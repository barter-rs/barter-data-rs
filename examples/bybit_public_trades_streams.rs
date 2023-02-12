use barter_integration::model::InstrumentKind;
use futures::StreamExt;
use barter_data::exchange::bybit::{spot::BybitSpot, futures::BybitFuturePerpetual};
use barter_data::streams::Streams;
use barter_data::subscription::trade::PublicTrades;
use tracing::info;

#[rustfmt::skip]
#[tokio::main]
async fn main() {
    init_logging();

    let streams = Streams::<PublicTrades>::builder()
        .subscribe([
            (BybitSpot::default(), "btc", "usdt", InstrumentKind::Spot, PublicTrades),
            (BybitSpot::default(), "eth", "usdt", InstrumentKind::Spot, PublicTrades),
        ])
        .subscribe([
            (BybitFuturePerpetual::default(), "btc", "usdt", InstrumentKind::FuturePerpetual, PublicTrades),
        ])
        .init()
        .await
        .unwrap();

    let mut joined_stream = streams.join_map().await;

    while let Some((exchange, trade)) = joined_stream.next().await {
        info!("Exchange: {exchange}, MarketEvent<PublicTrade>: {trade:?}");
    }

}

fn init_logging() {
    tracing_subscriber::fmt()
        // Filter messages based on the INFO
        .with_env_filter(
            tracing_subscriber::filter::EnvFilter::builder()
                .with_default_directive(tracing_subscriber::filter::LevelFilter::DEBUG.into())
                .from_env_lossy(),
        )
        .with_ansi(cfg!(debug_assertions))
        .json()
        .init()
}