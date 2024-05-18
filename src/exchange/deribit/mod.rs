use crate::exchange::deribit::channel::DeribitChannel;
use crate::exchange::deribit::market::DeribitMarket;
use crate::exchange::subscription::ExchangeSub;
use crate::exchange::{Connector, ExchangeId};
use crate::subscriber::validator::WebSocketSubValidator;
use crate::subscriber::WebSocketSubscriber;
use barter_integration::error::SocketError;
use barter_integration::model::SubscriptionId;
use barter_integration::protocol::websocket::WsMessage;
use barter_integration::Validator;
use barter_macro::{DeExchange, SerExchange};
use serde::{Deserialize, Serialize};
use url::Url;

pub mod channel;
pub mod market;
pub mod subscription;

/// [`Kraken`] server base url.
///
/// See docs: <https://docs.kraken.com/websockets/#overview>
pub const BASE_URL_DERIBIT: &str = "www.deribit.com";

/// [`Deribit`] exchange.
///
/// See docs: <https://docs.deribit.com/#overview>
#[derive(
    Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Default, DeExchange, SerExchange,
)]
pub struct Deribit;

impl Connector for Deribit {
    const ID: ExchangeId = ExchangeId::Deribit;
    type Channel = DeribitChannel;
    type Market = DeribitMarket;
    type Subscriber = WebSocketSubscriber;
    type SubValidator = WebSocketSubValidator;
    type SubResponse = DeribitResponse<Vec<SubscriptionId>>; // Todo:

    fn url() -> Result<Url, SocketError> {
        Url::parse(BASE_URL_DERIBIT).map_err(SocketError::UrlParse)
    }

    fn requests(
        exchange_subs: Vec<ExchangeSub<Self::Channel, Self::Market>>,
    ) -> impl IntoIterator<Item = WsMessage> {
        [
            WsMessage::try_from(&DeribitRequest::<DeribitSubParams>::from_iter(
                exchange_subs,
            ))
            .expect("failed to serialise DeribitRequest<DeribitSubParams>"),
        ]
    }
}

impl FromIterator<ExchangeSub<DeribitChannel, DeribitMarket>>
    for DeribitRequest<'_, DeribitSubParams>
{
    fn from_iter<T>(iter: T) -> Self
    where
        T: IntoIterator<Item = ExchangeSub<DeribitChannel, DeribitMarket>>,
    {
        Self {
            jsonrpc: "2.0",
            id: 0,
            method: "public/subscribe",
            params: DeribitSubParams {
                channels: iter
                    .into_iter()
                    .map(|ExchangeSub { channel, market }| {
                        format!("{}{}.raw", channel.as_ref(), market.as_ref())
                    })
                    .collect::<Vec<_>>(),
            },
        }
    }
}

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Deserialize, Serialize)]
pub struct DeribitRequest<'a, T> {
    jsonrpc: &'a str,
    id: u32,
    method: &'a str,
    params: T,
}

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Deserialize, Serialize)]
pub struct DeribitSubParams {
    channels: Vec<String>,
}

impl<'a, T> TryFrom<&'a DeribitRequest<'a, T>> for WsMessage
where
    T: Serialize,
{
    type Error = SocketError;

    fn try_from(value: &'a DeribitRequest<'a, T>) -> Result<Self, Self::Error> {
        serde_json::to_string(value)
            .map(WsMessage::Text)
            .map_err(SocketError::Serialise)
    }
}

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Deserialize, Serialize)]
pub struct DeribitResponse<T> {
    id: u32,
    result: T,
}

impl Validator for DeribitResponse<Vec<SubscriptionId>> {
    fn validate(self) -> Result<Self, SocketError>
    where
        Self: Sized,
    {
        if self.result.is_empty() {
            Err(SocketError::Subscribe(format!(
                "received failure subscription response: {self:?}",
            )))
        } else {
            Ok(self)
        }
    }
}
