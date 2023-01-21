use barter_data::{
    exchange::{binance::spot::BinanceSpot, kraken::Kraken},
    streams::Streams,
    subscription::book::OrderBooksL1,
};
use barter_integration::model::InstrumentKind;
use futures::StreamExt;
use tracing::info;

#[rustfmt::skip]
#[tokio::main]
async fn main() {
    // Initialise INFO Tracing log subscriber
    init_logging();

    // Initialise OrderBooksL1 Streams for various exchnages
    // '--> each call to StreamBuilder::subscribe() initialises a separate WebSocket connection
    let streams = Streams::builder()
        .subscribe([
            (BinanceSpot::default(), "btc", "usdt", InstrumentKind::Spot, OrderBooksL1),
            (BinanceSpot::default(), "eth", "usd", InstrumentKind::Spot, OrderBooksL1),
        ])
        .subscribe([
            (Kraken::default(), "xbt", "usd", InstrumentKind::Spot, OrderBooksL1),
            (Kraken::default(), "ada", "usd", InstrumentKind::Spot, OrderBooksL1),
            (Kraken::default(), "matic", "usd", InstrumentKind::Spot, OrderBooksL1),
            (Kraken::default(), "dot", "usd", InstrumentKind::Spot, OrderBooksL1),
        ])
        .init()
        .await
        .unwrap();

    // Join all exchange OrderBooksL1 streams into a single tokio_stream::StreamMap
    // Notes:
    //  - Use `streams.select(ExchangeId)` to interact with the individual exchange streams!
    //  - Use `streams.join()` to join all exchange streams into a single mpsc::UnboundedReceiver!
    let mut joined_stream = streams.join_map().await;

    while let Some((exchange, order_book_l1)) = joined_stream.next().await {
        info!("Exchange: {exchange}, Market<OrderBookL1>: {order_book_l1:?}");
    }
}

// Initialise an INFO `Subscriber` for `Tracing` Json logs and install it as the global default.
fn init_logging() {
    tracing_subscriber::fmt()
        // Filter messages based on the INFO
        .with_env_filter(
            tracing_subscriber::filter::EnvFilter::builder()
                .with_default_directive(tracing_subscriber::filter::LevelFilter::INFO.into())
                .from_env_lossy(),
        )
        // Disable colours on release builds
        .with_ansi(cfg!(debug_assertions))
        // Enable Json formatting
        .json()
        // Install this Tracing subscriber as global default
        .init()
}
