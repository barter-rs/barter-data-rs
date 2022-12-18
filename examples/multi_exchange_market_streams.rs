use barter_data::exchange::coinbase::Coinbase;
use barter_data::subscriber::subscription::trade::PublicTrades;
use barter_data::subscriber::subscription::{SubKind, Subscription};
use barter_data::{Identifier, MarketStream, StreamSelector};
use barter_integration::model::InstrumentKind;
use futures::StreamExt;

#[tokio::main]
async fn main() {
    // Subscriptions
    let subscriptions = vec![
        (Coinbase, "btc", "usd", InstrumentKind::Spot, PublicTrades).into(),
        (Coinbase, "eth", "usd", InstrumentKind::Spot, PublicTrades).into(),
        (Coinbase, "btc", "gbp", InstrumentKind::Spot, PublicTrades).into(),
        (Coinbase, "eth", "gbp", InstrumentKind::Spot, PublicTrades).into(),
        (Coinbase, "sol", "usdt", InstrumentKind::Spot, PublicTrades).into(),
    ];

    tokio::spawn(consume(subscriptions)).await.unwrap();
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
