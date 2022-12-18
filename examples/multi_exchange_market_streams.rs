use futures::StreamExt;
use barter_data::exchange::coinbase::Coinbase;
use barter_data::{Identifier, MarketStream, StreamSelector};
use barter_data::subscriber::subscription::{SubKind, Subscription};
use barter_data::subscriber::subscription::trade::PublicTrades;
use barter_integration::model::InstrumentKind;

#[tokio::main]
async fn main() {
    // Subscriptions
    let subscriptions = vec![
        (Coinbase, "btc", "usd", InstrumentKind::Spot, PublicTrades).into(),
        (Coinbase, "eth", "usd", InstrumentKind::Spot, PublicTrades).into(),
    ];

    let handle = tokio::spawn(consume(subscriptions))
        .await
        .unwrap();

    // let handle = tokio::spawn(test(subscriptions))
    //     .await
    //     .unwrap();

}

pub async fn consume<Exchange, Kind>(subscriptions: Vec<Subscription<Exchange, Kind>>)
where
    Exchange: StreamSelector<Kind>,
    Kind: SubKind,
    Subscription<Exchange, Kind>: Identifier<Exchange::Channel> + Identifier<Exchange::Market>,
{
    let mut stream = Exchange::Stream::init(&subscriptions)
        .await
        .unwrap();

    while let Some(event) = stream.next().await {
        println!("Consumed: {event:?}");
    }
}


// pub async fn we_want_this_api<Exchange, Kind>(subscriptions: Vec<Subscription<Exchange, Kind>>)
// where
//     Exchange: StreamSelector<Kind>,
// {
//     let mut stream = Exchange::Stream::init(&subscriptions)
//         .await
//         .unwrap();
//
//     while let Some(event) = stream.next().await {
//         println!("Consumed: {event:?}");
//     }
// }

// pub async fn consume<Exchange, Kind>(subscriptions: Vec<Subscription<Exchange, Kind>>)
// where
//     Exchange: Connector<Kind> + TransformerConstructor<Kind> + Send + Sync,
//     Kind: SubKind + Send + Sync,
//     Subscription<Exchange, Kind>: Identifier<Exchange::Channel> + Identifier<Exchange::Market>,
// {
//     let mut stream = ExchangeWsStream::init(&subscriptions)
//         .await
//         .unwrap();
//
//     while let Some(event) = stream.next().await {
//         println!("Consumed: {event:?}");
//     }
// }


