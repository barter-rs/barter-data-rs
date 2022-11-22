use barter_data::{builder::Streams, model::{
    subscription::{Interval, SubKind},
    MarketEvent,
}, ExchangeId, ExchangeWsStream};
use barter_integration::model::{Instrument, InstrumentKind};
use futures::StreamExt;
use barter_data::builder::consume;
use barter_data::exchange::binance::futures::BinanceFuturesUsd;
use barter_data::model::PublicTrade;

// StreamBuilder subscribing to various Futures & Spot MarketStreams from Ftx, Kraken,
// BinanceFuturesUsd & Coinbase
#[tokio::main]
async fn main() {
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



    // Create channel for this ExchangeId stream
    let (exchange_tx, exchange_rx) = tokio::sync::mpsc::unbounded_channel();

    let instrument = Instrument::from(("btc", "usdt", InstrumentKind::FuturePerpetual));
    tokio::spawn(
        consume::<ExchangeWsStream<BinanceFuturesUsd<PublicTrade>, PublicTrade>>(
            ExchangeId::BinanceFuturesUsd,
            (ExchangeId::BinanceFuturesUsd, instrument, PublicTrade)
        )
    )

    while let Some((exchange, event)) = joined_stream.next().await {
        println!("Exchange: {}, MarketEvent: {:?}", exchange, event);
    }
}
