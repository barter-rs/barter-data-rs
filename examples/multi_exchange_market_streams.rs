use barter_data::{
    builder::Streams,
    model::{Interval, MarketEvent, SubKind},
    ExchangeId,
};
use barter_integration::model::InstrumentKind;
use futures::StreamExt;

// StreamBuilder subscribing to various Futures & Spot MarketStreams from Ftx, Kraken & BinanceFuturesUsd
#[tokio::main]
async fn main() {
    init_logging();

    // Initialise `PublicTrade` & `Candle``MarketStreams` for `BinanceFuturesUsd`, `Ftx` & `Kraken`
    let streams = Streams::builder()
        // .subscribe_exchange(
        //     ExchangeId::Ftx,
        //     [
        //         ("btc", "usdt", InstrumentKind::FuturePerpetual, SubKind::Trade),
        //         ("eth", "usdt", InstrumentKind::FuturePerpetual, SubKind::Trade),
        //         ("btc", "usdt", InstrumentKind::Spot, SubKind::Trade),
        //         ("eth", "usdt", InstrumentKind::Spot, SubKind::Trade),
        //     ],
        // )
        .subscribe([
            // (ExchangeId::Kraken, "xbt", "usd", InstrumentKind::Spot, SubKind::Trade),
            // (ExchangeId::Kraken, "xbt", "usd", InstrumentKind::Spot, SubKind::Candle(Interval::Minute1)),
            (ExchangeId::Kraken, "eth", "usd", InstrumentKind::Spot, SubKind::Candle(Interval::Minute1)),
            // (ExchangeId::BinanceFuturesUsd, "btc", "usdt", InstrumentKind::FuturePerpetual, SubKind::Trade),
            // (ExchangeId::BinanceFuturesUsd, "eth", "usdt", InstrumentKind::FuturePerpetual, SubKind::Trade),
            // (ExchangeId::BinanceFuturesUsd, "eth", "usdt", InstrumentKind::FuturePerpetual, SubKind::OrderBookL2),
            // (ExchangeId::Coinbase, "btc", "usd", InstrumentKind::Spot, SubKind::OrderBookL2),
            // (ExchangeId::Coinbase, "btc", "usd", InstrumentKind::Spot, SubKind::Trade),
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

fn init_logging() {
    tracing_subscriber::fmt()
        // Filter messages based on the `RUST_LOG` environment variable
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        // Disable colours on release builds
        .with_ansi(cfg!(debug_assertions))
        // Enable Json formatting
        .json()
        // Install this Tracing subscriber as global default
        .init()
}
