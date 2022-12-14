use futures::StreamExt;
use barter_data::{ExchangeWsStream, Identifier, MarketStream};
use barter_data::exchange::coinbase::Coinbase;
use barter_data::exchange::{Connector, ExchangeId};
use barter_data::subscriber::subscription::trade::PublicTrades;
use barter_data::subscriber::validator::WebSocketSubValidator;
use barter_data::subscriber::{Subscriber, WebSocketSubscriber};
use barter_data::subscriber::subscription::{SubKind, Subscription};
use barter_data::transformer::{StatelessTransformer, TransformerConstructor};
use barter_integration::model::InstrumentKind;
use barter_integration::Transformer;

#[tokio::main]
async fn main() {
    // Subscriptions
    // let subscriptions = vec![
    //     (ExchangeId::Coinbase, "btc", "usd", InstrumentKind::Spot, PublicTrades).into(),
    //     (ExchangeId::Coinbase, "eth", "usd", InstrumentKind::Spot, PublicTrades).into(),
    // ];

    // let handle = tokio::spawn(consume::<Coinbase, _>(subscriptions))
    //     .await
    //     .unwrap();

}

// pub async fn consume<Exchange, Kind>(subscriptions: Vec<Subscription<Exchange, Kind>>)
// where
//     Exchange: Connector<Kind> + TransformerConstructor<Kind>,
//     Kind: SubKind + Send + Sync,
//     Subscription<Exchange, Kind>: Identifier<Exchange::Channel> + Identifier<Exchange::Market>,
// {
//     let mut stream: ExchangeWsStream<Exchange::Transformer> = ExchangeWsStream::init::<Exchange>(&subscriptions)
//         .await
//         .unwrap();
//
//
// }