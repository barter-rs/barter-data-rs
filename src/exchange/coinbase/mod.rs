use crate::{
    model::subscription::{SubKind, SubscriptionIds, SubscriptionMeta},
    ExchangeId, ExchangeTransformer, MarketEvent, Subscriber, Subscription, Validator,
};
use barter_integration::{
    error::SocketError, model::SubscriptionId, protocol::websocket::WsMessage, Transformer,
};
use model::{CoinbaseMessage, CoinbaseSubResponse};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;
use tokio::sync::mpsc;

/// [`Coinbase`] specific data structures.
mod model;

/// [`Coinbase`] [`Subscriber`] & [`ExchangeTransformer`] implementor for the collection
/// of `Spot` & `Futures` data.
#[derive(Clone, Eq, PartialEq, Debug, Deserialize, Serialize)]
pub struct Coinbase {
    pub ids: SubscriptionIds,
}

impl Subscriber for Coinbase {
    type SubResponse = CoinbaseSubResponse;

    fn base_url() -> &'static str {
        "wss://ws-feed.exchange.coinbase.com"
    }

    fn build_subscription_meta(
        subscriptions: &[Subscription],
    ) -> Result<SubscriptionMeta, SocketError> {
        // Allocate SubscriptionIds HashMap to track identifiers for each actioned Subscription
        let mut ids = SubscriptionIds(HashMap::with_capacity(subscriptions.len()));

        // Map Barter Subscriptions to Coinbase channels
        let subscriptions = subscriptions
            .iter()
            .map(|subscription| {
                // Determine the Coinbase specific channel & market for this Barter Subscription
                let (channel, market) = Self::build_channel_meta(subscription)?;

                // Use "channel|market" as the SubscriptionId key in the SubscriptionIds HashMap
                // eg/ SubscriptionId("matches|ETH-USD")
                ids.insert(
                    Coinbase::subscription_id(channel, &market),
                    subscription.clone(),
                );

                // Construct Coinbase specific subscription message
                Ok(Self::subscription(channel, &market))
            })
            .collect::<Result<Vec<_>, SocketError>>()?;

        Ok(SubscriptionMeta {
            ids,
            expected_responses: subscriptions.len(),
            subscriptions,
        })
    }
}

impl ExchangeTransformer for Coinbase {
    const EXCHANGE: ExchangeId = ExchangeId::Coinbase;
    fn new(_: mpsc::UnboundedSender<WsMessage>, ids: SubscriptionIds) -> Self {
        Self { ids }
    }
}

impl Transformer<MarketEvent> for Coinbase {
    type Input = CoinbaseMessage;
    type OutputIter = Vec<Result<MarketEvent, SocketError>>;

    fn transform(&mut self, input: Self::Input) -> Self::OutputIter {
        match input {
            CoinbaseMessage::Trade(trade) => {
                match self.ids.find_instrument(&trade.subscription_id) {
                    Ok(instrument) => vec![Ok(MarketEvent::from((
                        Coinbase::EXCHANGE,
                        instrument,
                        trade,
                    )))],
                    Err(error) => vec![Err(error)],
                }
            }
        }
    }
}

impl Coinbase {
    /// [`Coinbase`] trades channel name.
    ///
    /// See docs: <https://docs.cloud.coinbase.com/exchange/docs/websocket-channels#match>
    pub const CHANNEL_TRADES: &'static str = "matches";

    /// Determine the [`Coinbase`] channel metadata associated with an input Barter [`Subscription`].
    /// This includes the [`Coinbase`] &str channel, and a `String` market identifier. Both are
    /// used to build an [`Coinbase`] subscription payload.
    ///
    /// Example Ok return: Ok("matches", "BTC-USD")
    /// where channel == "matches" & market == "BTC-USD".
    pub fn build_channel_meta(sub: &Subscription) -> Result<(&str, String), SocketError> {
        // Validate provided Subscription InstrumentKind is supported by Coinbase
        let sub = sub.validate()?;

        // Determine Coinbase channel using the Subscription SubKind
        let channel = match &sub.kind {
            SubKind::Trade => Self::CHANNEL_TRADES,
            other => {
                return Err(SocketError::Unsupported {
                    entity: Self::EXCHANGE.as_str(),
                    item: other.to_string(),
                })
            }
        };

        // Determine Coinbase market identifier using the Instrument (eg/ "BTC-USD")
        let market = format!("{}-{}", sub.instrument.base, sub.instrument.quote).to_uppercase();

        Ok((channel, market))
    }

    /// Build a [`Coinbase`] compatible subscription message using the channel & market provided.
    pub fn subscription(channel: &str, market: &str) -> WsMessage {
        WsMessage::Text(
            json!({
                "type": "subscribe",
                "product_ids": [market],
                "channels": [channel],
            })
            .to_string(),
        )
    }

    /// Build a [`Coinbase`] compatible [`SubscriptionId`] using the channel & market provided.
    /// This is used to associate [`Coinbase`] data structures received over the WebSocket with it's
    /// original Barter [`Subscription`].
    ///
    /// eg/ SubscriptionId("matches|ETH-USD")
    pub fn subscription_id(channel: &str, market: &str) -> SubscriptionId {
        SubscriptionId::from(format!("{channel}|{market}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::exchange::coinbase::model::CoinbaseTrade;
    use crate::model::{subscription::Interval, DataKind, PublicTrade};
    use barter_integration::model::{Exchange, Instrument, InstrumentKind, Side};
    use chrono::Utc;

    fn coinbase(subscriptions: Vec<Subscription>) -> Coinbase {
        let ids = SubscriptionIds(
            subscriptions
                .into_iter()
                .map(|sub| {
                    let subscription_id = match (&sub.kind, &sub.instrument.kind) {
                        (SubKind::Trade, InstrumentKind::Spot) => {
                            let product_id =
                                format!("{}-{}", sub.instrument.base, sub.instrument.quote)
                                    .to_uppercase();
                            format!("matches|{}", product_id)
                        }
                        (_, _) => {
                            panic!("not supported")
                        }
                    };

                    (SubscriptionId::from(subscription_id), sub)
                })
                .collect(),
        );

        Coinbase { ids }
    }

    #[test]
    fn test_build_channel_meta() {
        struct TestCase {
            input: Subscription,
            expected: Result<(&'static str, String), SocketError>,
        }

        let cases = vec![
            TestCase {
                // TC0: Supported InstrumentKind::Spot trades subscription
                input: Subscription::new(
                    ExchangeId::Coinbase,
                    ("btc", "usd", InstrumentKind::Spot),
                    SubKind::Trade,
                ),
                expected: Ok(("matches", "BTC-USD".to_owned())),
            },
            TestCase {
                // TC1: Unsupported InstrumentKind::Spot candle subscription
                input: Subscription::new(
                    ExchangeId::Coinbase,
                    ("btc", "usd", InstrumentKind::Spot),
                    SubKind::Candle(Interval::Minute5),
                ),
                expected: Err(SocketError::Unsupported {
                    entity: "",
                    item: "".to_string(),
                }),
            },
            TestCase {
                // TC2: Unsupported InstrumentKind::FuturePerpetual trades subscription
                input: Subscription::new(
                    ExchangeId::Coinbase,
                    ("btc", "usd", InstrumentKind::FuturePerpetual),
                    SubKind::Trade,
                ),
                expected: Err(SocketError::Unsupported {
                    entity: "",
                    item: "".to_string(),
                }),
            },
        ];

        for (index, test) in cases.into_iter().enumerate() {
            let actual = Coinbase::build_channel_meta(&test.input);
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
    fn test_coinbase_transform() {
        let mut transformer = coinbase(vec![Subscription::from((
            ExchangeId::Coinbase,
            "btc",
            "usd",
            InstrumentKind::Spot,
            SubKind::Trade,
        ))]);

        let time = Utc::now();

        struct TestCase {
            input: CoinbaseMessage,
            expected: Vec<Result<MarketEvent, SocketError>>,
        }

        let cases = vec![
            TestCase {
                // TC0: CoinbaseMessage Spot trades w/ known SubscriptionId
                input: CoinbaseMessage::Trade(CoinbaseTrade {
                    subscription_id: SubscriptionId::from("matches|BTC-USD"),
                    id: 2,
                    price: 1.0,
                    quantity: 1.0,
                    side: Side::Buy,
                    time,
                }),
                expected: vec![Ok(MarketEvent {
                    exchange_time: time,
                    received_time: time,
                    exchange: Exchange::from(ExchangeId::Coinbase),
                    instrument: Instrument::from(("btc", "usd", InstrumentKind::Spot)),
                    kind: DataKind::Trade(PublicTrade {
                        id: "2".to_string(),
                        price: 1.0,
                        quantity: 1.0,
                        side: Side::Buy,
                    }),
                })],
            },
            TestCase {
                // TC1: CoinbaseMessage with unknown SubscriptionId
                input: CoinbaseMessage::Trade(CoinbaseTrade {
                    subscription_id: SubscriptionId::from("unknown"),
                    id: 1,
                    price: 1.0,
                    quantity: 1.0,
                    side: Side::Buy,
                    time,
                }),
                expected: vec![Err(SocketError::Unidentifiable(SubscriptionId::from(
                    "unknown",
                )))],
            },
        ];

        for (index, test) in cases.into_iter().enumerate() {
            let actual = transformer.transform(test.input);
            assert_eq!(
                actual.len(),
                test.expected.len(),
                "TestCase {} failed at vector length assert_eq with actual: {:?}",
                index,
                actual
            );

            for (vector_index, (actual, expected)) in actual
                .into_iter()
                .zip(test.expected.into_iter())
                .enumerate()
            {
                match (actual, expected) {
                    (Ok(actual), Ok(expected)) => {
                        // Scrub Utc::now() timestamps to allow comparison
                        let actual = MarketEvent {
                            received_time: time,
                            ..actual
                        };
                        assert_eq!(
                            actual, expected,
                            "TC{} failed at vector index {}",
                            index, vector_index
                        )
                    }
                    (Err(_), Err(_)) => {
                        // Test passed
                    }
                    (actual, expected) => {
                        // Test failed
                        panic!("TC{index} failed at vector index {vector_index} because actual != expected. \nActual: {actual:?}\nExpected: {expected:?}\n");
                    }
                }
            }
        }
    }
}
