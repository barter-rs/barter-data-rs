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

// StreamBuilder subscribing to various Futures & Spot MarketStreams from Ftx, Kraken,
// BinanceFuturesUsd & Coinbase
#[tokio::main]
async fn main() {
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
