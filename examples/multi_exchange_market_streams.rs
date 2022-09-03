use barter_data::{
    builder::Streams,
    model::{Interval, MarketEvent, SubKind},
    ExchangeId,
};
use barter_integration::model::InstrumentKind;
use futures::StreamExt;

/// [`StreamBuilder`] subscribing to Binance Futures, Ftx and Ftx Futures PublicTrades for several
/// market [`Instrument`]s.
#[tokio::main]
async fn main() {
    // Initialise `PublicTrade` `MarketStreams` for `BinanceFuturesUsd` & `Ftx`
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
        .subscribe_exchange(
            ExchangeId::Kraken,
            [
                ("xbt", "usd", InstrumentKind::Spot, SubKind::Trade),
                ("xbt", "usd", InstrumentKind::Spot, SubKind::Candle(Interval::Minute1)),
            ],
        )
        .subscribe([
            (ExchangeId::Ftx, "xrp", "usdt", InstrumentKind::FuturePerpetual, SubKind::Trade),
            (ExchangeId::BinanceFuturesUsd, "btc", "usdt", InstrumentKind::FuturePerpetual, SubKind::Trade),
            (ExchangeId::BinanceFuturesUsd, "eth", "usdt", InstrumentKind::FuturePerpetual, SubKind::Trade),
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
