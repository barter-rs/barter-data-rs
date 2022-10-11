use crate::{
    model::{
        subscription::{SubKind, Subscription, SubscriptionIds, SubscriptionMeta},
        MarketEvent,
    },
    ExchangeId, ExchangeTransformer, Subscriber,
};
use barter_integration::{
    error::SocketError,
    model::{InstrumentKind, SubscriptionId},
    protocol::websocket::WsMessage,
    Transformer, Validator,
};
use model::{FtxMessage, FtxSubResponse};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;
use tokio::sync::mpsc;

/// [`Ftx`] specific data structures.
pub mod model;

/// `Ftx` [`Subscriber`] & [`ExchangeTransformer`] implementor for the collection
/// of `Spot` & `Futures` data.
#[derive(Clone, Eq, PartialEq, Debug, Deserialize, Serialize)]
pub struct Ftx {
    pub ids: SubscriptionIds,
}

impl Subscriber for Ftx {
    type SubResponse = FtxSubResponse;

    fn base_url() -> &'static str {
        "wss://ftx.com/ws/"
    }

    fn build_subscription_meta(
        subscriptions: &[Subscription],
    ) -> Result<SubscriptionMeta, SocketError> {
        // Allocate SubscriptionIds HashMap to track identifiers for each actioned Subscription
        let mut ids = SubscriptionIds(HashMap::with_capacity(subscriptions.len()));

        // Map Barter Subscriptions to Ftx subscriptions
        let subscriptions = subscriptions
            .iter()
            .map(|subscription| {
                // Determine the Ftx specific channel & market for this Barter Subscription
                let (channel, market) = Self::build_channel_meta(subscription)?;

                // Use "channel|market" as the SubscriptionId key in the SubscriptionIds
                // eg/ SubscriptionId("trades|BTC/USDT")
                ids.insert(Ftx::subscription_id(channel, &market), subscription.clone());

                // Construct Ftx specific subscription message
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

impl ExchangeTransformer for Ftx {
    const EXCHANGE: ExchangeId = ExchangeId::Ftx;
    fn new(_: mpsc::UnboundedSender<WsMessage>, ids: SubscriptionIds) -> Self {
        Self { ids }
    }
}

impl Transformer<MarketEvent> for Ftx {
    type Input = FtxMessage;
    type OutputIter = Vec<Result<MarketEvent, SocketError>>;

    fn transform(&mut self, input: Self::Input) -> Self::OutputIter {
        match input {
            FtxMessage::Trades {
                subscription_id,
                trades,
            } => {
                let instrument = match self.ids.find_instrument(&subscription_id) {
                    Ok(instrument) => instrument,
                    Err(error) => return vec![Err(error)],
                };

                trades
                    .into_iter()
                    .map(|trade| {
                        Ok(MarketEvent::from((
                            Ftx::EXCHANGE,
                            instrument.clone(),
                            trade,
                        )))
                    })
                    .collect()
            }
        }
    }
}

impl Ftx {
    /// [`Ftx`] trades channel name.
    ///
    /// See docs: <https://docs.ftx.com/#trades>
    pub const CHANNEL_TRADES: &'static str = "trades";

    /// Determine the [`Ftx`] channel metadata associated with an input Barter [`Subscription`].
    /// This includes the [`Ftx`] `&str` channel identifier, and a `String` market identifier. Both
    /// are used to build an [`Ftx`] subscription payload.
    ///
    /// Example Ok Return: Ok("trades", "BTC/USDT")
    /// where channel == "trades" & market == "BTC/USDT"
    pub fn build_channel_meta(sub: &Subscription) -> Result<(&str, String), SocketError> {
        // Validate provided Subscription InstrumentKind is supported by Ftx
        let sub = sub.validate()?;

        // Determine Ftx channel using the Subscription SubKind
        let channel = match &sub.kind {
            SubKind::Trade => Self::CHANNEL_TRADES,
            other => {
                return Err(SocketError::Unsupported {
                    entity: Self::EXCHANGE.as_str(),
                    item: other.to_string(),
                })
            }
        };

        // Determine Ftx market using the Instrument
        let market = match &sub.instrument.kind {
            InstrumentKind::Spot => format!("{}/{}", sub.instrument.base, sub.instrument.quote),
            InstrumentKind::FuturePerpetual => format!("{}-PERP", sub.instrument.base),
        };

        Ok((channel, market.to_uppercase()))
    }

    /// Build a [`Ftx`] compatible subscription message using the channel & market provided.
    pub fn subscription(channel: &str, market: &str) -> WsMessage {
        WsMessage::Text(
            json!({
                "op": "subscribe",
                "channel": channel,
                "market": market,
            })
            .to_string(),
        )
    }

    /// Build a [`Ftx`] compatible [`SubscriptionId`] using the channel & market provided.
    /// This is used to associate [`Ftx`] data structures received over the WebSocket with it's
    /// original Barter [`Subscription`].
    ///
    /// eg/ SubscriptionId("trades|BTC/USDT")
    pub fn subscription_id(channel: &str, market: &str) -> SubscriptionId {
        SubscriptionId::from(format!("{channel}|{market}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::exchange::ftx::model::FtxTrade;
    use crate::model::{subscription::Interval, DataKind, PublicTrade};
    use barter_integration::model::{Exchange, Instrument, Side};
    use chrono::Utc;

    fn ftx(subscriptions: Vec<Subscription>) -> Ftx {
        let ids = SubscriptionIds(
            subscriptions
                .into_iter()
                .map(|sub| {
                    let subscription_id = match (&sub.kind, &sub.instrument.kind) {
                        (SubKind::Trade, InstrumentKind::Spot) => Ftx::subscription_id(
                            Ftx::CHANNEL_TRADES,
                            &format!("{}/{}", sub.instrument.base, sub.instrument.quote)
                                .to_uppercase(),
                        ),
                        (SubKind::Trade, InstrumentKind::FuturePerpetual) => Ftx::subscription_id(
                            Ftx::CHANNEL_TRADES,
                            &format!("{}-PERP", sub.instrument.base).to_uppercase(),
                        ),
                        (_, _) => {
                            panic!("not supported")
                        }
                    };

                    (subscription_id, sub)
                })
                .collect(),
        );

        Ftx { ids }
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
                    ExchangeId::Ftx,
                    ("btc", "usdt", InstrumentKind::Spot),
                    SubKind::Trade,
                ),
                expected: Ok(("trades", "BTC/USDT".to_owned())),
            },
            TestCase {
                // TC1: Supported InstrumentKind::FuturePerpetual trades subscription
                input: Subscription::new(
                    ExchangeId::Ftx,
                    ("btc", "usdt", InstrumentKind::FuturePerpetual),
                    SubKind::Trade,
                ),
                expected: Ok(("trades", "BTC-PERP".to_owned())),
            },
            TestCase {
                // TC2: Unsupported InstrumentKind::FuturePerpetual candle subscription
                input: Subscription::new(
                    ExchangeId::Ftx,
                    ("btc", "usdt", InstrumentKind::FuturePerpetual),
                    SubKind::Candle(Interval::Minute5),
                ),
                expected: Err(SocketError::Unsupported {
                    entity: "",
                    item: "".to_string(),
                }),
            },
        ];

        for (index, test) in cases.into_iter().enumerate() {
            let actual = Ftx::build_channel_meta(&test.input);
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
    fn test_ftx_transform() {
        let mut transformer = ftx(vec![
            Subscription::from((
                ExchangeId::Ftx,
                "btc",
                "usdt",
                InstrumentKind::Spot,
                SubKind::Trade,
            )),
            Subscription::from((
                ExchangeId::Ftx,
                "btc",
                "usdt",
                InstrumentKind::FuturePerpetual,
                SubKind::Trade,
            )),
        ]);

        let time = Utc::now();

        struct TestCase {
            input: FtxMessage,
            expected: Vec<Result<MarketEvent, SocketError>>,
        }

        let cases = vec![
            TestCase {
                // TC0: FtxMessage with unknown SubscriptionId
                input: FtxMessage::Trades {
                    subscription_id: SubscriptionId::from("unknown"),
                    trades: vec![],
                },
                expected: vec![Err(SocketError::Unidentifiable(SubscriptionId::from(
                    "unknown",
                )))],
            },
            TestCase {
                // TC1: FtxMessage Spot trades w/ known SubscriptionId
                input: FtxMessage::Trades {
                    subscription_id: SubscriptionId::from("trades|BTC/USDT"),
                    trades: vec![
                        FtxTrade {
                            id: 1,
                            price: 1.0,
                            size: 1.0,
                            side: Side::Buy,
                            time: time,
                        },
                        FtxTrade {
                            id: 2,
                            price: 1.0,
                            size: 1.0,
                            side: Side::Sell,
                            time: time,
                        },
                    ],
                },
                expected: vec![
                    Ok(MarketEvent {
                        exchange_time: time,
                        received_time: time,
                        exchange: Exchange::from(ExchangeId::Ftx),
                        instrument: Instrument::from(("btc", "usdt", InstrumentKind::Spot)),
                        kind: DataKind::Trade(PublicTrade {
                            id: "1".to_string(),
                            price: 1.0,
                            quantity: 1.0,
                            side: Side::Buy,
                        }),
                    }),
                    Ok(MarketEvent {
                        exchange_time: time,
                        received_time: time,
                        exchange: Exchange::from(ExchangeId::Ftx),
                        instrument: Instrument::from(("btc", "usdt", InstrumentKind::Spot)),
                        kind: DataKind::Trade(PublicTrade {
                            id: "2".to_string(),
                            price: 1.0,
                            quantity: 1.0,
                            side: Side::Sell,
                        }),
                    }),
                ],
            },
            TestCase {
                // TC2: FtxMessage FuturePerpetual trades w/ known SubscriptionId
                input: FtxMessage::Trades {
                    subscription_id: SubscriptionId::from("trades|BTC-PERP"),
                    trades: vec![
                        FtxTrade {
                            id: 1,
                            price: 1.0,
                            size: 1.0,
                            side: Side::Buy,
                            time: time,
                        },
                        FtxTrade {
                            id: 2,
                            price: 1.0,
                            size: 1.0,
                            side: Side::Sell,
                            time: time,
                        },
                    ],
                },
                expected: vec![
                    Ok(MarketEvent {
                        exchange_time: time,
                        received_time: time,
                        exchange: Exchange::from(ExchangeId::Ftx),
                        instrument: Instrument::from((
                            "btc",
                            "usdt",
                            InstrumentKind::FuturePerpetual,
                        )),
                        kind: DataKind::Trade(PublicTrade {
                            id: "1".to_string(),
                            price: 1.0,
                            quantity: 1.0,
                            side: Side::Buy,
                        }),
                    }),
                    Ok(MarketEvent {
                        exchange_time: time,
                        received_time: time,
                        exchange: Exchange::from(ExchangeId::Ftx),
                        instrument: Instrument::from((
                            "btc",
                            "usdt",
                            InstrumentKind::FuturePerpetual,
                        )),
                        kind: DataKind::Trade(PublicTrade {
                            id: "2".to_string(),
                            price: 1.0,
                            quantity: 1.0,
                            side: Side::Sell,
                        }),
                    }),
                ],
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
