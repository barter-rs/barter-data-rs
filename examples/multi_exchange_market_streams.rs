use barter_data::{builder::Streams, ExchangeTransformerId};
use barter_integration::{InstrumentKind, StreamKind};
use futures::StreamExt;

/// [`StreamBuilder`] subscribing to Binance Futures, Ftx and Ftx Futures Trades for several
/// market [`Instrument`]s.
#[tokio::main]
async fn main() {
    // Initialise `Trade` `MarketStreams` for `BinanceFutures` & `Ftx`
    let streams = Streams::builder()
        .subscribe(
            ExchangeTransformerId::BinanceFutures,
            [
                (
                    "btc",
                    "usdt",
                    InstrumentKind::FuturePerpetual,
                    StreamKind::Trade,
                ),
                (
                    "eth",
                    "usdt",
                    InstrumentKind::FuturePerpetual,
                    StreamKind::Trade,
                ),
                (
                    "xrp",
                    "usdt",
                    InstrumentKind::FuturePerpetual,
                    StreamKind::Trade,
                ),
            ],
        )
        .subscribe(
            ExchangeTransformerId::Ftx,
            [
                ("btc", "usdt", InstrumentKind::Spot, StreamKind::Trade),
                ("eth", "usdt", InstrumentKind::Spot, StreamKind::Trade),
                ("xrp", "usdt", InstrumentKind::Spot, StreamKind::Trade),
                (
                    "btc",
                    "usdt",
                    InstrumentKind::FuturePerpetual,
                    StreamKind::Trade,
                ),
                (
                    "eth",
                    "usdt",
                    InstrumentKind::FuturePerpetual,
                    StreamKind::Trade,
                ),
                (
                    "xrp",
                    "usdt",
                    InstrumentKind::FuturePerpetual,
                    StreamKind::Trade,
                ),
            ],
        )
        .init()
        .await
        .unwrap();

    // Join all exchange streams into a StreamMap
    // Note: Use `streams.select(ExchangeTransformerId)` to interact with individual streams!
    let mut joined_stream = streams.join().await;

    while let Some((exchange, event)) = joined_stream.next().await {
        println!("Exchange: {}, MarketData: {:?}", exchange, event);
    }
}
