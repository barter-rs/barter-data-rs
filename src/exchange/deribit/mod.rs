use crate::exchange::deribit::channel::DeribitChannel;
use crate::exchange::deribit::market::DeribitMarket;
use crate::exchange::deribit::request::{DeribitRequest, DeribitSubParams};
use crate::exchange::deribit::subscription::DeribitResponse;
use crate::exchange::deribit::trade::DeribitTrades;
use crate::exchange::subscription::ExchangeSub;
use crate::exchange::{Connector, ExchangeId, StreamSelector};
use crate::instrument::InstrumentData;
use crate::subscriber::validator::WebSocketSubValidator;
use crate::subscriber::WebSocketSubscriber;
use crate::subscription::trade::PublicTrades;
use crate::transformer::stateless::StatelessTransformer;
use crate::ExchangeWsStream;
use barter_integration::error::SocketError;
use barter_integration::model::SubscriptionId;
use barter_integration::protocol::websocket::WsMessage;
use barter_macro::{DeExchange, SerExchange};
use url::Url;

pub mod channel;
pub mod market;
pub mod message;
pub mod request;
pub mod subscription;
pub mod trade;

/// [`Kraken`] server base url.
///
/// See docs: <https://docs.kraken.com/websockets/#overview>
pub const BASE_URL_DERIBIT: &str = "wss://www.deribit.com/ws/api/v2";

/// [`Deribit`] exchange.
///
/// See docs: <https://docs.deribit.com/#overview>
#[derive(
    Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Default, DeExchange, SerExchange,
)]
pub struct Deribit;

impl Connector for Deribit {
    const ID: ExchangeId = ExchangeId::Deribit;
    type Channel = DeribitChannel<'static>;
    type Market = DeribitMarket;
    type Subscriber = WebSocketSubscriber;
    type SubValidator = WebSocketSubValidator;
    type SubResponse = DeribitResponse<Vec<String>>;

    fn url() -> Result<Url, SocketError> {
        Url::parse(BASE_URL_DERIBIT).map_err(SocketError::UrlParse)
    }

    fn requests(
        exchange_subs: Vec<ExchangeSub<Self::Channel, Self::Market>>,
    ) -> impl IntoIterator<Item = WsMessage> {
        let x = [WsMessage::Text(
            serde_json::to_string(&DeribitRequest::<DeribitSubParams>::from_iter(
                exchange_subs,
            ))
            .expect("failed to serialise DeribitRequest<DeribitSubParams>"),
        )];

        let y = x.iter().cloned().collect::<Vec<_>>();

        println!("{}", y[0].to_string());
        x
    }
}

impl<Instrument> StreamSelector<Instrument, PublicTrades> for Deribit
where
    Instrument: InstrumentData,
{
    type Stream =
        ExchangeWsStream<StatelessTransformer<Self, Instrument::Id, PublicTrades, DeribitTrades>>;
}

// impl<Instrument> StreamSelector<Instrument, OrderBooksL1> for Deribit
// where
//     Instrument: InstrumentData,
// {
//     type Stream = ExchangeWsStream<StatelessTransformer<Self, Instrument::Id, OrderBooksL1, DeribitOrderBookL1>>;
// }
