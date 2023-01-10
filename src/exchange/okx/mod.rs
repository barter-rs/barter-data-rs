use self::{
    channel::OkxChannel, market::OkxMarket, subscription::OkxSubResponse, trade::OkxTrades,
};
use crate::{
    exchange::{Connector, ExchangeId, ExchangeSub},
    subscriber::{validator::WebSocketSubValidator, WebSocketSubscriber},
    subscription::trade::PublicTrades,
    transformer::stateless::StatelessTransformer,
    ExchangeWsStream, StreamSelector,
};
use barter_integration::{error::SocketError, protocol::websocket::WsMessage};
use barter_macro::{DeExchange, SerExchange};
use serde_json::json;
use url::Url;

/// Todo:
pub mod channel;
pub mod market;
pub mod subscription;
pub mod trade;

/// [`Okx`] server base url.
///
/// See docs: <https://www.okx.com/docs-v5/en/#overview-api-resources-and-support>
pub const BASE_URL_OKX: &str = "wss://wsaws.okx.com:8443/ws/v5/public";

/// Todo:
///
/// See docs: <https://www.okx.com/docs-v5/en/#websocket-api>
#[derive(
    Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Default, DeExchange, SerExchange,
)]
pub struct Okx;

impl Connector for Okx {
    const ID: ExchangeId = ExchangeId::Okx;
    type Channel = OkxChannel;
    type Market = OkxMarket;
    type Subscriber = WebSocketSubscriber<Self::SubValidator>;
    type SubValidator = WebSocketSubValidator;
    type SubResponse = OkxSubResponse;

    fn url() -> Result<Url, SocketError> {
        Url::parse(BASE_URL_OKX).map_err(SocketError::UrlParse)
    }

    fn requests(exchange_subs: Vec<ExchangeSub<Self::Channel, Self::Market>>) -> Vec<WsMessage> {
        vec![WsMessage::Text(
            json!({
                "op": "subscribe",
                "args": &exchange_subs,
            })
            .to_string(),
        )]
    }
}

impl StreamSelector<PublicTrades> for Okx {
    type Stream = ExchangeWsStream<StatelessTransformer<Self, PublicTrades, OkxTrades>>;
}

// impl<'de> serde::Deserialize<'de> for Okx {
//     fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
//     where
//         D: serde::de::Deserializer<'de>,
//     {
//         match <String as serde::Deserialize>::deserialize(deserializer)?.as_str() {
//             "Okx" | "okx" => Ok(Self),
//             other => Err(serde::de::Error::invalid_value(
//                 serde::de::Unexpected::Str(other),
//                 &"Okx | okx",
//             )),
//         }
//     }
// }

// impl<'de> serde::Deserialize<'de> for Okx {
//     fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
//     where
//         D: serde::de::Deserializer<'de>,
//     {
//         struct ExchangeVisitor;
//
//         impl<'de> Visitor<'de> for ExchangeVisitor {
//             type Value = Okx;
//
//             fn expecting(&self, formatter: &mut Formatter) -> std::fmt::Result {
//                 todo!()
//             }
//
//             fn visit_str<E>(self, v: &str) -> Result<Self::Value, E> where E: Error {
//                 match v {
//                     "Okx" | "okx" => Ok(Okx),
//                     other => Err(serde::de::Error::invalid_value(
//                         serde::de::Unexpected::Str(other),
//                         &"Okx | okx",
//                     )),
//                 }
//             }
//
//             fn visit_string<E>(self, v: String) -> Result<Self::Value, E> where E: Error {
//                 match v.as_str() {
//                     "Okx" | "okx" => Ok(Okx),
//                     other => Err(serde::de::Error::invalid_value(
//                         serde::de::Unexpected::Str(other),
//                         &"Okx | okx",
//                     )),
//                 }
//             }
//         }
//
//         deserializer.deserialize_any(ExchangeVisitor)
//     }
// }

#[test]
fn it_works() {
    #[derive(serde::Deserialize, PartialEq, Debug)]
    struct Dummy<T> {
        exchange: T,
    }

    let input = r#"
        {
            "exchange": "okx"
        }
    "#;

    let actual = serde_json::from_str::<Dummy<Okx>>(input).unwrap();
    println!("{actual:?}");

    assert_eq!(actual, Dummy { exchange: Okx });

    let actual = serde_json::to_string(&Okx).unwrap();
    println!("{actual}");
}

// impl serde::Serialize for Okx {
//     fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
//     where
//         S: serde::ser::Serializer,
//     {
//         serializer.serialize_str(Okx::ID.as_str())
//     }
// }
