use crate::{
    Identifier,
    subscriber::subscription::{
        trade::PublicTrades,
        liquidation::Liquidations,
        ExchangeSubscription, SubKind, Subscription, SubscriptionMap
    },
};
use barter_integration::{
    error::SocketError, model::SubscriptionId, protocol::websocket::WsMessage, Validator,
};
use serde::{Deserialize, Serialize};

/// Todo:
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Serialize)]
pub struct BinanceChannel(pub &'static str);

impl BinanceChannel {
    /// Binance real-time trades channel name.
    ///
    /// See docs: <https://binance-docs.github.io/apidocs/spot/en/#trade-streams>
    ///
    /// Note:
    /// - For [`BinanceFuturesUsd`] this real-time stream is undocumented.
    /// See discord: <https://discord.com/channels/910237311332151317/923160222711812126/975712874582388757>
    pub const TRADES: Self = Self("@trade");

    /// [`BinanceFuturesUsd`] liquidation orders channel name.
    ///
    /// See docs: <https://binance-docs.github.io/apidocs/futures/en/#liquidation-order-streams>
    pub const LIQUIDATIONS: Self = Self("@forceOrder");
}

impl Identifier<BinanceChannel> for Subscription<PublicTrades> {
    fn id(&self) -> BinanceChannel {
        BinanceChannel::TRADES
    }
}

impl Identifier<BinanceChannel> for Subscription<Liquidations> {
    fn id(&self) -> BinanceChannel {
        BinanceChannel::LIQUIDATIONS
    }
}

/// Todo:
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Serialize)]
pub struct BinanceSubMeta {
    channel: BinanceChannel,
    market: String,
}

impl Identifier<SubscriptionId> for BinanceSubMeta {
    fn id(&self) -> SubscriptionId {
        subscription_id(self.channel, &self.market)
    }
}

impl<ExchangeEvent> ExchangeSubscription<ExchangeEvent> for BinanceSubMeta
where
    ExchangeEvent: Identifier<Option<SubscriptionId>> + for<'de> Deserialize<'de>,
{
    type Channel = BinanceChannel;
    type SubResponse = BinanceSubResponse;

    fn new<Kind>(sub: &Subscription<Kind>) -> Self
    where
        Kind: SubKind,
        Subscription<Kind>: Identifier<Self::Channel>,
    {
        Self {
            channel: sub.id(),
            market: format!("{}{}", sub.instrument.base, sub.instrument.quote),
        }
    }

    fn requests(subscriptions: Vec<Self>) -> Vec<WsMessage> {
        let stream_names = subscriptions
            .into_iter()
            .map(|meta| format!("{}{}", meta.market, meta.channel.0))
            .collect::<Vec<String>>();

        vec![WsMessage::Text(
            serde_json::json!({
                "method": "SUBSCRIBE",
                "params": stream_names,
                "id": 1
            })
            .to_string(),
        )]
    }

    fn expected_responses<Kind>(_: &SubscriptionMap<Kind>) -> usize {
        1
    }
}

/// Generate a [`BinanceSpot`] & [`BinanceFuturesUsd`] [`SubscriptionId`] from the channel and
/// market provided.
///
/// Uses "channel|MARKET" (uppercase market is used in order to match incoming exchange events):
/// eg/ SubscriptionId("@aggTrade|BTCUSDT")
pub(crate) fn subscription_id(channel: BinanceChannel, market: &str) -> SubscriptionId {
    SubscriptionId::from(format!("{}|{}", channel.0, market.to_uppercase()))
}

/// Binance subscription response message.
///
/// See docs: <https://binance-docs.github.io/apidocs/spot/en/#live-subscribing-unsubscribing-to-streams>
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Deserialize, Serialize)]
pub struct BinanceSubResponse {
    result: Option<Vec<String>>,
    id: u32,
}

impl Validator for BinanceSubResponse {
    fn validate(self) -> Result<Self, SocketError>
    where
        Self: Sized,
    {
        if self.result.is_none() {
            Ok(self)
        } else {
            Err(SocketError::Subscribe(
                "received failure subscription response".to_owned(),
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::exchange::binance::*;
    use crate::exchange::binance::domain::subscription::BinanceSubResponse;

    #[test]
    fn test_deserialise_binance_subscription_response() {
        struct TestCase {
            input: &'static str,
            expected: Result<BinanceSubResponse, SocketError>,
        }

        let cases = vec![
            TestCase {
                // TC0: input response is Subscribed
                input: r#"{"id":1,"result":null}"#,
                expected: Ok(BinanceSubResponse {
                    result: None,
                    id: 1,
                }),
            },
            TestCase {
                // TC1: input response is failed subscription
                input: r#"{"result": [], "id": 1}"#,
                expected: Ok(BinanceSubResponse {
                    result: Some(vec![]),
                    id: 1,
                }),
            },
        ];

        for (index, test) in cases.into_iter().enumerate() {
            let actual = serde_json::from_str::<BinanceSubResponse>(test.input);
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
    fn test_validate_binance_subscription_response() {
        struct TestCase {
            input_response: BinanceSubResponse,
            is_valid: bool,
        }

        let cases = vec![
            TestCase {
                // TC0: input response is successful subscription
                input_response: BinanceSubResponse {
                    result: None,
                    id: 1,
                },
                is_valid: true,
            },
            TestCase {
                // TC1: input response is failed subscription
                input_response: BinanceSubResponse {
                    result: Some(vec![]),
                    id: 1,
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
