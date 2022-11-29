// use barter_data::{
//     builder::Streams,
//     model::{
//         MarketEvent,
//         subscription::{Interval, SubKind},
//     },
// };
// use barter_integration::model::InstrumentKind;
// use futures::StreamExt;
// use barter_data::exchange::ExchangeId;

use futures::StreamExt;
use barter_data::exchange::binance::futures::BinanceFuturesUsd;
use barter_data::exchange::binance::domain::trade::BinanceTrade;
use barter_data::{ExchangeTransformer, ExchangeWsStream};
use barter_data::exchange::binance::futures::liquidation::BinanceLiquidation;
use barter_data::exchange::binance::spot::BinanceSpot;
use barter_data::exchange::coinbase::pro::CoinbasePro;
use barter_data::exchange::coinbase::trade::CoinbaseTrade;
use barter_data::exchange::ExchangeId;
use barter_data::exchange::kraken::domain::trade::{KrakenTrade, KrakenTrades};
use barter_data::exchange::kraken::Kraken;
use barter_data::exchange::okx::domain::trade::OkxTrades;
use barter_data::exchange::okx::Okx;
use barter_data::model::PublicTrade;
use barter_data::subscriber::subscription::Subscription;
use barter_data::subscriber::{Subscriber, WebSocketSubscriber};
use barter_data::subscriber::subscription::liquidation::Liquidations;
use barter_data::subscriber::subscription::trade::PublicTrades;
use barter_integration::model::{Instrument, InstrumentKind};

// StreamBuilder subscribing to various Futures & Spot MarketStreams from Ftx, Kraken,
// BinanceFuturesUsd & Coinbase
#[tokio::main]
async fn main() {
    // Initialise Tracing subscriber
    init_logging();

    // Subscribers
    // let subscriber: WebSocketSubscriber<BinanceSpot, PublicTrades, BinanceTrade> = WebSocketSubscriber::new();
    // let subscriber: WebSocketSubscriber<BinanceFuturesUsd, PublicTrades, BinanceTrade> = WebSocketSubscriber::new();
    // let subscriber: WebSocketSubscriber<BinanceFuturesUsd, Liquidations, BinanceLiquidation> = WebSocketSubscriber::new();
    // let subscriber: WebSocketSubscriber<CoinbasePro, PublicTrades, CoinbaseTrade> = WebSocketSubscriber::new();
    // let subscriber: WebSocketSubscriber<Kraken, PublicTrades, KrakenTrades> = WebSocketSubscriber::new();
    let subscriber: WebSocketSubscriber<Okx, PublicTrades, OkxTrades> = WebSocketSubscriber::new();

    // Subscriptions
    let subscriptions = vec![
        // (ExchangeId::BinanceSpot, "btc", "usdt", InstrumentKind::Spot, PublicTrades).into(),
        // (ExchangeId::BinanceSpot, "eth", "usdt", InstrumentKind::Spot, PublicTrades).into(),
        // (ExchangeId::BinanceFuturesUsd, "btc", "usdt", InstrumentKind::FuturePerpetual, PublicTrades).into(),
        // (ExchangeId::BinanceFuturesUsd, "eth", "usdt", InstrumentKind::FuturePerpetual, PublicTrades).into(),
        // (ExchangeId::BinanceFuturesUsd, "btc", "usdt", InstrumentKind::FuturePerpetual, Liquidations).into(),
        // (ExchangeId::BinanceFuturesUsd, "eth", "usdt", InstrumentKind::FuturePerpetual, Liquidations).into()
        // (ExchangeId::CoinbasePro, "btc", "usd", InstrumentKind::Spot, PublicTrades).into(),
        // (ExchangeId::CoinbasePro, "eth", "usd", InstrumentKind::Spot, PublicTrades).into(),
        // (ExchangeId::Kraken, "btc", "usd", InstrumentKind::Spot, PublicTrades).into(),
        // (ExchangeId::Kraken, "eth", "usd", InstrumentKind::Spot, PublicTrades).into(),
        // (ExchangeId::Okx, "btc", "usdt", InstrumentKind::Spot, PublicTrades).into(),
        (ExchangeId::Okx, "btc", "usdt", InstrumentKind::FuturePerpetual, PublicTrades).into(),
        (ExchangeId::Okx, "eth", "usdt", InstrumentKind::FuturePerpetual, PublicTrades).into(),
    ];

    let (websocket, subscription_map) = subscriber
        .subscribe(&subscriptions)
        .await
        .expect("failed to subscribe");

    let (_, ws_stream) = websocket.split();

    // let transformer: ExchangeTransformer<BinanceSpot, PublicTrades, BinanceTrade> = ExchangeTransformer::new(subscription_map);
    // let transformer: ExchangeTransformer<BinanceFuturesUsd, PublicTrades, BinanceTrade> = ExchangeTransformer::new(subscription_map);
    // let transformer: ExchangeTransformer<BinanceFuturesUsd, Liquidations, BinanceLiquidation> = ExchangeTransformer::new(subscription_map);
    // let transformer: ExchangeTransformer<CoinbasePro, PublicTrades, CoinbaseTrade> = ExchangeTransformer::new(subscription_map);
    // let transformer: ExchangeTransformer<Kraken, PublicTrades, KrakenTrades> = ExchangeTransformer::new(subscription_map);
    let transformer: ExchangeTransformer<Okx, PublicTrades, OkxTrades> = ExchangeTransformer::new(subscription_map);


    let mut ws_stream = ExchangeWsStream::new(ws_stream, transformer);
    while let Some(event) = ws_stream.next().await {
        println!("{:?}", event);
    }

    // Todo:
    // // Initialise a `PublicTrade`, `Candle` & `OrderBook``MarketStream` for
    // // `BinanceFuturesUsd`, `Ftx`, `Kraken` & `Coinbase`
    // let streams = Streams::builder()
    //     .subscribe_exchange(
    //         ExchangeId::Ftx,
    //         [
    //             ("btc", "usdt", InstrumentKind::FuturePerpetual, SubKind::Trade),
    //             ("eth", "usdt", InstrumentKind::FuturePerpetual, SubKind::Trade),
    //             ("btc", "usdt", InstrumentKind::Spot, SubKind::Trade),
    //             ("eth", "usdt", InstrumentKind::Spot, SubKind::Trade),
    //         ],
    //     )
    //     .subscribe([
    //         (ExchangeId::Coinbase, "btc", "usd", InstrumentKind::Spot, SubKind::Trade),
    //         (ExchangeId::Coinbase, "eth", "usd", InstrumentKind::Spot, SubKind::Trade),
    //         (ExchangeId::Kraken, "xbt", "usd", InstrumentKind::Spot, SubKind::Trade),
    //         (ExchangeId::Kraken, "xbt", "usd", InstrumentKind::Spot, SubKind::Candle(Interval::Minute1)),
    //         (ExchangeId::BinanceFuturesUsd, "btc", "usdt", InstrumentKind::FuturePerpetual, SubKind::Trade),
    //         (ExchangeId::BinanceFuturesUsd, "eth", "usdt", InstrumentKind::FuturePerpetual, SubKind::Trade),
    //         (ExchangeId::BinanceFuturesUsd, "btc", "usdt", InstrumentKind::FuturePerpetual, SubKind::OrderBook),
    //     ])
    //     .init()
    //     .await
    //     .unwrap();
    //
    // // Join all exchange streams into a StreamMap
    // // Note: Use `streams.select(ExchangeId)` to interact with the individual exchange streams!
    // let mut joined_stream = streams.join_map::<MarketEvent>().await;
    //
    // while let Some((exchange, event)) = joined_stream.next().await {
    //     println!("Exchange: {}, MarketEvent: {:?}", exchange, event);
    // }
}

// Initialise a `Subscriber` for `Tracing` JSON logs.
fn init_logging() {
    tracing_subscriber::fmt()
        // Filter messages based on the `RUST_LOG` environment variable
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        // Disable colours on release builds
        .with_ansi(cfg!(debug_assertions))
        // Enable Json formatting
        .json()
        // Install this Tracing subscriber as global default
        .init();
}