use crate::{
    subscriber::subscription::{trade::PublicTrades, ExchangeSubscription, SubKind, Subscription},
    Identifier,
};
use barter_integration::{
    error::SocketError,
    model::{InstrumentKind, SubscriptionId},
    protocol::websocket::WsMessage,
    Validator,
};
use serde::{Deserialize, Serialize};
use serde_json::json;

/// Todo:
///
/// See docs: <https://www.okx.com/docs-v5/en/#websocket-api-public-channel>
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Serialize)]
pub struct OkxChannel(pub &'static str);

impl OkxChannel {
    /// [`Okx`] real-time trades channel.
    ///
    /// See docs: <https://www.okx.com/docs-v5/en/#websocket-api-public-channel-trades-channel>
    pub const TRADES: Self = Self("trades");
}

impl Identifier<OkxChannel> for Subscription<PublicTrades> {
    fn id(&self) -> OkxChannel {
        OkxChannel::TRADES
    }
}

/// Todo:
///
/// See docs: <https://www.okx.com/docs-v5/en/#websocket-api-public-channel>
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Serialize)]
pub struct OkxSubMeta {
    channel: OkxChannel,
    #[serde(rename = "instId")]
    market: String,
}

impl Identifier<SubscriptionId> for OkxSubMeta {
    fn id(&self) -> SubscriptionId {
        subscription_id(self.channel.0, &self.market)
    }
}

impl<ExchangeEvent> ExchangeSubscription<ExchangeEvent> for OkxSubMeta
where
    ExchangeEvent: Identifier<Option<SubscriptionId>> + for<'de> Deserialize<'de>,
{
    type Channel = OkxChannel;
    type SubResponse = OkxSubResponse;

    fn new<Kind>(sub: &Subscription<Kind>) -> Self
    where
        Kind: SubKind,
        Subscription<Kind>: Identifier<Self::Channel>,
    {
        Self {
            channel: sub.id(),
            market: match sub.instrument.kind {
                InstrumentKind::Spot => {
                    format!("{}-{}", sub.instrument.base, sub.instrument.quote).to_uppercase()
                }
                InstrumentKind::FuturePerpetual => {
                    format!("{}-{}-SWAP", sub.instrument.base, sub.instrument.quote).to_uppercase()
                }
            },
        }
    }

    fn requests(subscriptions: Vec<Self>) -> Vec<WsMessage> {
        vec![WsMessage::Text(
            json!({
                "op": "subscribe",
                "args": &subscriptions,
            })
            .to_string(),
        )]
    }
}

/// Generate an [`Okx`] [`SubscriptionId`] from the channel and market provided.
///
/// Uses "channel|MARKET":
/// eg/ SubscriptionId("trades|ETH-USD")
pub(crate) fn subscription_id(channel: &str, market: &str) -> SubscriptionId {
    SubscriptionId::from(format!("{}|{}", channel, market))
}

/// [`Okx`] WebSocket subscription response.
///
/// ## Examples
/// ### Subscription Trades Ok Response
/// ```json
/// {
///   "event": "subscribe",
///   "args": {
///     "channel": "trades",
///     "instId": "BTC-USD-191227"
///   }
/// }
/// ```
///
/// ### Subscription Trades Error Response
/// ```json
/// {
///   "event": "error",
///   "code": "60012",
///   "msg": "Invalid request: {\"op\": \"subscribe\", \"args\":[{ \"channel\" : \"trades\", \"instId\" : \"BTC-USD-191227\"}]}"
/// }
/// ```
///
/// See docs: <https://www.okx.com/docs-v5/en/#websocket-api-subscribe>
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Deserialize, Serialize)]
#[serde(tag = "event", rename_all = "lowercase")]
pub enum OkxSubResponse {
    #[serde(rename = "subscribe")]
    Subscribed,
    Error {
        code: String,
        #[serde(rename = "msg")]
        message: String,
    },
}

impl Validator for OkxSubResponse {
    fn validate(self) -> Result<Self, SocketError>
    where
        Self: Sized,
    {
        match self {
            Self::Subscribed => Ok(self),
            Self::Error { code, message } => Err(SocketError::Subscribe(format!(
                "received failure subscription response code: {code} with message: {message}",
            ))),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::exchange::okx::domain::subscription::OkxSubResponse;
    use crate::exchange::okx::domain::*;
    use barter_integration::error::SocketError;

    #[test]
    fn test_de_okx_subscription_response() {
        struct TestCase {
            input: &'static str,
            expected: Result<OkxSubResponse, SocketError>,
        }

        let cases = vec![
            TestCase {
                // TC0: input response is subscription success
                input: r#"
                {
                    "event": "subscribe",
                    "args": {"channel": "trades", "instId": "BTC-USD-191227"}
                }
                "#,
                expected: Ok(OkxSubResponse::Subscribed),
            },
            TestCase {
                // TC1: input response is failed subscription
                input: r#"
                {
                    "event": "error",
                    "code": "60012",
                    "msg": "Invalid request: {\"op\": \"subscribe\", \"args\":[{ \"channel\" : \"trades\", \"instId\" : \"BTC-USD-191227\"}]}"
                }
                "#,
                expected: Ok(OkxSubResponse::Error {
                    code: "60012".to_string(),
                    message: "Invalid request: {\"op\": \"subscribe\", \"args\":[{ \"channel\" : \"trades\", \"instId\" : \"BTC-USD-191227\"}]}".to_string()
                }),
            },
        ];

        for (index, test) in cases.into_iter().enumerate() {
            let actual = serde_json::from_str::<OkxSubResponse>(test.input);
            match (actual, test.expected) {
                (Ok(actual), Ok(expected)) => {
                    assert_eq!(actual, expected, "TC{} failed", index)
                }
                (Err(_), Err(_)) => {
                    // Test passed
                }
                (actual, expected) => {
                    // Test failed
                    panic!("TC{index} failed because actual != expected. \nActual: {actual:?}\nExpected: {expected:?}\n");
                }
            }
        }
    }

    #[test]
    fn test_validate_coinbase_subscription_response() {
        struct TestCase {
            input_response: OkxSubResponse,
            is_valid: bool,
        }

        let cases = vec![
            TestCase {
                // TC0: input response is subscription success
                input_response: OkxSubResponse::Subscribed,
                is_valid: true,
            },
            TestCase {
                // TC1: input response is failed subscription
                input_response: OkxSubResponse::Error {
                    code: "60012".to_string(),
                    message: "Invalid request: {\"op\": \"subscribe\", \"args\":[{ \"channel\" : \"trades\", \"instId\" : \"BTC-USD-191227\"}]}".to_string()
                },
                is_valid: false,
            },
        ];

        for (index, test) in cases.into_iter().enumerate() {
            let actual = test.input_response.validate().is_ok();
            assert_eq!(actual, test.is_valid, "TestCase {} failed", index);
        }
    }
}
