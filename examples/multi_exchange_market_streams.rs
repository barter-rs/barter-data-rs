use barter_data::exchange::binance::futures::BinanceFuturesUsd;
use barter_data::exchange::binance::spot::{BinanceServerSpot, BinanceSpot};
use barter_data::exchange::binance::Binance;
use barter_data::exchange::coinbase::Coinbase;
use barter_data::subscriber::subscription::trade::PublicTrades;
use barter_data::subscriber::subscription::{SubKind, Subscription};
use barter_data::{Identifier, MarketStream, StreamSelector};
use barter_integration::model::InstrumentKind;
use futures::StreamExt;
use barter_data::exchange::okx::Okx;

#[tokio::main]
async fn main() {
    // Initialise Tracing log subscriber (uses INFO filter if RUST_LOG env var is not set)
    init_logging();

    // Subscriptions
    let subscriptions = vec![
        // (Coinbase, "btc", "usd", InstrumentKind::Spot, PublicTrades).into(),
        // (Coinbase, "eth", "usd", InstrumentKind::Spot, PublicTrades).into(),
        // (Coinbase, "btc", "gbp", InstrumentKind::Spot, PublicTrades).into(),
        // (Coinbase, "eth", "gbp", InstrumentKind::Spot, PublicTrades).into(),
        // (Coinbase, "sol", "usdt", InstrumentKind::Spot, PublicTrades).into(),
        (Okx, "btc", "usdt", InstrumentKind::FuturePerpetual, PublicTrades).into(),
        (Okx, "eth", "usdt", InstrumentKind::FuturePerpetual, PublicTrades).into(),
        // (
        //     BinanceSpot::default(),
        //     "btc",
        //     "usdt",
        //     InstrumentKind::Spot,
        //     PublicTrades,
        // )
        //     .into(),
        // (
        //     BinanceSpot::default(),
        //     "eth",
        //     "usdt",
        //     InstrumentKind::Spot,
        //     PublicTrades,
        // )
        //     .into(),
        // (
        //     BinanceSpot::default(),
        //     "xrp",
        //     "usdt",
        //     InstrumentKind::Spot,
        //     PublicTrades,
        // )
        //     .into(),
    ];

    tokio::spawn(consume(subscriptions)).await.unwrap();
}

/// Initialise a `Subscriber` for `Tracing` Json logs and install it as the global default.
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

pub async fn consume<Exchange, Kind>(subscriptions: Vec<Subscription<Exchange, Kind>>)
where
    Exchange: StreamSelector<Kind>,
    Kind: SubKind,
    Subscription<Exchange, Kind>: Identifier<Exchange::Channel> + Identifier<Exchange::Market>,
{
    let mut stream = Exchange::Stream::init(&subscriptions).await.unwrap();

    while let Some(event) = stream.next().await {
        println!("Consumed: {event:?}");
    }
}
