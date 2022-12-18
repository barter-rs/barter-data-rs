use barter_data::{
    builder::Streams,
    model::{
        subscription::{Interval, SubKind},
        MarketEvent,
    },
    ExchangeId,
};
use barter_integration::model::InstrumentKind;
use futures::StreamExt;

// StreamBuilder subscribing to various Futures & Spot MarketStreams from Ftx, Kraken,
// BinanceFuturesUsd & Coinbase
#[tokio::main]
async fn main() {
    // Initialise Tracing log subscriber (uses INFO filter if RUST_LOG env var is not set)
    init_logging();

    // Initialise a `PublicTrade`, `Candle` & `OrderBook``MarketStream` for
    // `BinanceFuturesUsd`, `Ftx`, `Kraken` & `Coinbase`
    let streams = Streams::builder()
        .subscribe_exchange(
            ExchangeId::Ftx,
            [
                ("btc", "usdt", InstrumentKind::FuturePerpetual, SubKind::Trade),
                ("eth", "usdt", InstrumentKind::FuturePerpetual, SubKind::Trade),
                ("btc", "usdt", InstrumentKind::Spot, SubKind::Trade),
                ("eth", "usdt", InstrumentKind::Spot, SubKind::Trade),
            ],
        )
        .subscribe([
            (ExchangeId::Coinbase, "btc", "usd", InstrumentKind::Spot, SubKind::Trade),
            (ExchangeId::Coinbase, "eth", "usd", InstrumentKind::Spot, SubKind::Trade),
            (ExchangeId::Kraken, "xbt", "usd", InstrumentKind::Spot, SubKind::Trade),
            (ExchangeId::Kraken, "xbt", "usd", InstrumentKind::Spot, SubKind::Candle(Interval::Minute1)),
            (ExchangeId::BinanceFuturesUsd, "btc", "usdt", InstrumentKind::FuturePerpetual, SubKind::Trade),
            (ExchangeId::BinanceFuturesUsd, "eth", "usdt", InstrumentKind::FuturePerpetual, SubKind::Trade),
            (ExchangeId::BinanceFuturesUsd, "btc", "usdt", InstrumentKind::FuturePerpetual, SubKind::OrderBook),
        ])
        .init()
        .await
        .unwrap();

    // Join all exchange streams into a StreamMap
    // Note: Use `streams.select(ExchangeId)` to interact with the individual exchange streams!
    let mut joined_stream = streams.join_map::<MarketEvent>().await;

    while let Some((exchange, event)) = joined_stream.next().await {
        println!("Exchange: {}, MarketEvent: {:?}", exchange, event);
    }
}

// Initialise a `Subscriber` for `Tracing` Json logs and install it as the global default.
fn init_logging() {
    use tracing_subscriber::EnvFilter;
    use tracing::metadata::LevelFilter;

    let environment_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(LevelFilter::INFO.to_string()));

    tracing_subscriber::fmt()
        // Filter messages based on the `RUST_LOG` environment variable
        .with_env_filter(environment_filter)

        // Disable colours on release builds
        .with_ansi(cfg!(debug_assertions))

        // Enable Json formatting
        .json()

        // Install this Tracing subscriber as global default
        .init()
}
