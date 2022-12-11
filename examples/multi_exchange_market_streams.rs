use futures::StreamExt;
use barter_data::exchange::coinbase::{Coinbase, CoinbaseSubMeta};
use barter_data::exchange::{Connector, ExchangeId};
use barter_data::subscriber::subscription::trade::PublicTrades;
use barter_data::subscriber::validator::WebSocketSubValidator;
use barter_data::subscriber::{Subscriber, WebSocketSubscriber};
use barter_integration::model::InstrumentKind;

// StreamBuilder subscribing to various Futures & Spot MarketStreams from Ftx, Kraken,
// BinanceFuturesUsd & Coinbase
#[tokio::main]
async fn main() {
    // Subscriptions
    let subscriptions = vec![
        // (ExchangeId::BinanceSpot, "btc", "usdt", InstrumentKind::Spot, PublicTrades).into(),
        // (ExchangeId::BinanceSpot, "eth", "usdt", InstrumentKind::Spot, PublicTrades).into(),
        // (ExchangeId::BinanceFuturesUsd, "btc", "usdt", InstrumentKind::FuturePerpetual, PublicTrades).into(),
        // (ExchangeId::BinanceFuturesUsd, "eth", "usdt", InstrumentKind::FuturePerpetual, PublicTrades).into(),
        // (ExchangeId::BinanceFuturesUsd, "btc", "usdt", InstrumentKind::FuturePerpetual, Liquidations).into(),
        // (ExchangeId::BinanceFuturesUsd, "eth", "usdt", InstrumentKind::FuturePerpetual, Liquidations).into()
        (ExchangeId::Coinbase, "btc", "usd", InstrumentKind::Spot, PublicTrades).into(),
        (ExchangeId::Coinbase, "eth", "usd", InstrumentKind::Spot, PublicTrades).into(),
        // (ExchangeId::Kraken, "xbt", "usd", InstrumentKind::Spot, PublicTrades).into(),
        // (ExchangeId::Kraken, "eth", "usd", InstrumentKind::Spot, PublicTrades).into(),
        // (ExchangeId::Kraken, "xrp", "usd", InstrumentKind::Spot, PublicTrades).into(),
        // (ExchangeId::Okx, "btc", "usdt", InstrumentKind::Spot, PublicTrades).into(),
        // (ExchangeId::Okx, "btc", "usdt", InstrumentKind::FuturePerpetual, PublicTrades).into(),
        // (ExchangeId::Okx, "eth", "usdt", InstrumentKind::FuturePerpetual, PublicTrades).into(),
        // (ExchangeId::GateioFuturesUsd, "btc", "usdt", InstrumentKind::FuturePerpetual, PublicTrades).into(),
        // (ExchangeId::GateioFuturesUsd, "eth", "usdt", InstrumentKind::FuturePerpetual, PublicTrades).into(),
        // (ExchangeId::GateioFuturesUsd, "xrp", "usdt", InstrumentKind::FuturePerpetual, PublicTrades).into(),
    ];


    let (websocket, map) = WebSocketSubscriber::<WebSocketSubValidator>::subscribe::<PublicTrades, Coinbase>(&subscriptions)
        .await
        .expect("failed to subscribe");

    let (_, mut ws_stream) = websocket.split();

    while let Some(event) = ws_stream.next().await {
        println!("Consumed: {:?}", event);
    }








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
