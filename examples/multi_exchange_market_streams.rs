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
use barter_data::exchange::binance::trade::BinanceTrade;
use barter_data::{ExchangeTransformer, ExchangeWsStream};
use barter_data::exchange::binance::futures::liquidation::BinanceLiquidation;
use barter_data::exchange::binance::spot::BinanceSpot;
use barter_data::exchange::ExchangeId;
use barter_data::model::PublicTrade;
use barter_data::subscriber::subscription::{Liquidations, PublicTrades, Subscription};
use barter_data::subscriber::{Subscriber, WebSocketSubscriber};
use barter_integration::model::{Instrument, InstrumentKind};

// StreamBuilder subscribing to various Futures & Spot MarketStreams from Ftx, Kraken,
// BinanceFuturesUsd & Coinbase
#[tokio::main]
async fn main() {
    // Subscribers
    // let subscriber: WebSocketSubscriber<BinanceSpot, PublicTrades, BinanceTrade> = WebSocketSubscriber::new();
    // let subscriber: WebSocketSubscriber<BinanceFuturesUsd, PublicTrades, BinanceTrade> = WebSocketSubscriber::new();
    let subscriber: WebSocketSubscriber<BinanceFuturesUsd, Liquidations, BinanceLiquidation> = WebSocketSubscriber::new();

    // Subscriptions
    let subscriptions = vec![
        // (ExchangeId::BinanceSpot, "btc", "usdt", InstrumentKind::Spot, PublicTrades).into(),
        // (ExchangeId::BinanceSpot, "eth", "usdt", InstrumentKind::Spot, PublicTrades).into(),

        // (ExchangeId::BinanceFuturesUsd, "btc", "usdt", InstrumentKind::FuturePerpetual, PublicTrades).into(),
        // (ExchangeId::BinanceFuturesUsd, "eth", "usdt", InstrumentKind::FuturePerpetual, PublicTrades).into(),
        //
        (ExchangeId::BinanceFuturesUsd, "btc", "usdt", InstrumentKind::FuturePerpetual, Liquidations).into(),
        (ExchangeId::BinanceFuturesUsd, "eth", "usdt", InstrumentKind::FuturePerpetual, Liquidations).into(),
    ];

    let (websocket, subscription_map) = subscriber
        .subscribe(&subscriptions)
        .await
        .expect("failed to subscribe");

    let (_, ws_stream) = websocket.split();

    // let transformer: ExchangeTransformer<BinanceSpot, PublicTrades, BinanceTrade> = ExchangeTransformer::new(subscription_map);
    // let transformer: ExchangeTransformer<BinanceFuturesUsd, PublicTrades, BinanceTrade> = ExchangeTransformer::new(subscription_map);
    let transformer: ExchangeTransformer<BinanceFuturesUsd, Liquidations, BinanceLiquidation> = ExchangeTransformer::new(subscription_map);


    let mut ws_stream = ExchangeWsStream::new(ws_stream, transformer);
    while let Some(event) = ws_stream.next().await {
        println!("{:?}", event.unwrap());
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
