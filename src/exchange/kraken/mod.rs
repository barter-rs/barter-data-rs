use crate::{
    ExchangeId, ExchangeTransformer, MarketEvent, Subscriber, Subscription, SubscriptionIds,
    SubscriptionMeta,
};
use barter_integration::{
    error::SocketError, model::SubscriptionId, protocol::websocket::WsMessage, Transformer,
};
use model::{KrakenEvent, KrakenMessage, KrakenSubKind, KrakenSubResponse, KrakenSubscription};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tokio::sync::mpsc;
use tracing::debug;

/// [`Kraken`] specific data structures.
pub mod model;

/// `Kraken` [`Subscriber`] & [`ExchangeTransformer`] implementor for the collection
/// of `Spot` data.
#[derive(Clone, Eq, PartialEq, Debug, Deserialize, Serialize)]
pub struct Kraken {
    pub ids: SubscriptionIds,
}

impl Subscriber for Kraken {
    type SubResponse = KrakenSubResponse;

    fn base_url() -> &'static str {
        "wss://ws.kraken.com/"
    }

    fn build_subscription_meta(
        subscriptions: &[Subscription],
    ) -> Result<SubscriptionMeta, SocketError> {
        // Allocate SubscriptionIds HashMap to track identifiers for each actioned Subscription
        let mut ids = SubscriptionIds(HashMap::with_capacity(subscriptions.len()));

        // Map Barter Subscriptions to Kraken Subscriptions
        let subscriptions = subscriptions
            .iter()
            .map(|subscription| {
                // Translate Barter Subscription to the associated KrakenSubscription
                let kraken_subscription = Kraken::subscription(subscription)?;

                // Use "channel|market" as the SubscriptionId key in the SubscriptionIds
                // eg/ SubscriptionId("ohlc-5|XBT/USD")
                ids.insert(
                    SubscriptionId::from(&kraken_subscription),
                    subscription.clone(),
                );

                WsMessage::try_from(&kraken_subscription)
            })
            .collect::<Result<Vec<_>, SocketError>>()?;

        Ok(SubscriptionMeta {
            ids,
            expected_responses: subscriptions.len(),
            subscriptions,
        })
    }
}

impl ExchangeTransformer for Kraken {
    const EXCHANGE: ExchangeId = ExchangeId::Kraken;

    fn new(_: mpsc::UnboundedSender<WsMessage>, ids: SubscriptionIds) -> Self {
        Self { ids }
    }
}

impl Transformer<MarketEvent> for Kraken {
    type Input = KrakenMessage;
    type OutputIter = Vec<Result<MarketEvent, SocketError>>;

    fn transform(&mut self, input: Self::Input) -> Self::OutputIter {
        match input {
            KrakenMessage::Trades(trades) => {
                // Determine Instrument associated with this KrakenTrades message
                let instrument = match self.ids.find_instrument(&trades.subscription_id) {
                    Ok(instrument) => instrument,
                    Err(error) => return vec![Err(error)],
                };

                // Map to MarketEvents
                trades
                    .trades
                    .into_iter()
                    .map(|trade| {
                        Ok(MarketEvent::from((
                            Kraken::EXCHANGE,
                            instrument.clone(),
                            trade,
                        )))
                    })
                    .collect()
            }
            KrakenMessage::Candle(candle) => {
                // Determine Instrument associated with this KrakenCandle message
                let instrument = match self.ids.find_instrument(&candle.subscription_id) {
                    Ok(instrument) => instrument,
                    Err(error) => return vec![Err(error)],
                };

                // Map to MarketEvent
                vec![Ok(MarketEvent::from((
                    Kraken::EXCHANGE,
                    instrument,
                    candle.candle,
                )))]
            }
            KrakenMessage::KrakenEvent(KrakenEvent::Heartbeat) => {
                debug!(exchange_id = %Kraken::EXCHANGE, "received heartbeat");
                vec![]
            }
            KrakenMessage::KrakenEvent(KrakenEvent::Error(error)) => {
                vec![Err(SocketError::Exchange(error.message))]
            }
        }
    }
}

impl Kraken {
    /// Translate a Barter [`Subscription`] into a [`Kraken`] compatible subscription message.
    pub fn subscription(sub: &Subscription) -> Result<KrakenSubscription, SocketError> {
        // Determine Kraken market identifier using the Instrument
        let market = format!("{}/{}", sub.instrument.base, sub.instrument.quote).to_uppercase();

        // Determine the KrakenSubKind from the Barter SubKind
        let kind = KrakenSubKind::try_from(&sub.kind)?;

        Ok(KrakenSubscription::new(market, kind))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::exchange::kraken::model::{
        KrakenCandle, KrakenCandleData, KrakenError, KrakenInterval, KrakenTrade, KrakenTrades,
    };
    use crate::model::{Candle, DataKind, Interval, PublicTrade, SubKind};
    use barter_integration::model::{Exchange, Instrument, InstrumentKind, Side};
    use chrono::{DateTime, Utc};

    fn kraken(subscriptions: Vec<Subscription>) -> Kraken {
        let ids = SubscriptionIds(
            subscriptions
                .into_iter()
                .map(|sub| {
                    if InstrumentKind::Spot != sub.instrument.kind {
                        panic!("non spot InstrumentKinds not supported by Kraken exchange")
                    }

                    let market =
                        format!("{}/{}", sub.instrument.base, sub.instrument.quote).to_uppercase();

                    let subscription_id = match &sub.kind {
                        SubKind::Trade => {
                            format!("trade|{market}")
                        }
                        SubKind::Candle(interval) => {
                            let interval = KrakenInterval::try_from(interval)
                                .expect("interval not supports by kraken");

                            format!("ohlc-{}|{market}", u32::from(interval))
                        }
                        _ => panic!(
                            "subscription type not implemented in mod tests kraken() builder"
                        ),
                    };

                    (SubscriptionId(subscription_id), sub)
                })
                .collect(),
        );

        Kraken { ids }
    }

    fn kraken_trade_id(timestamp: DateTime<Utc>, side: Side, price: f64, volume: f64) -> String {
        format!("{}_{:?}_{}_{}", timestamp, side, price, volume)
    }

    #[test]
    fn test_subscription() {
        struct TestCase {
            input: Subscription,
            expected: Result<KrakenSubscription, SocketError>,
        }

        let cases = vec![
            TestCase {
                // TC0: Valid InstrumentKind::Spot Trade Subscription
                input: Subscription::new(
                    ExchangeId::Kraken,
                    ("xbt", "usd", InstrumentKind::Spot),
                    SubKind::Trade,
                ),
                expected: Ok(KrakenSubscription {
                    event: "subscribe",
                    market: "XBT/USD".to_string(),
                    kind: KrakenSubKind::Trade { channel: "trade" },
                }),
            },
            TestCase {
                // TC1: Valid InstrumentKind::Spot Candle Subscription
                input: Subscription::new(
                    ExchangeId::Kraken,
                    ("xbt", "usd", InstrumentKind::Spot),
                    SubKind::Candle(Interval::Minute5),
                ),
                expected: Ok(KrakenSubscription {
                    event: "subscribe",
                    market: "XBT/USD".to_string(),
                    kind: KrakenSubKind::Candle {
                        channel: "ohlc",
                        interval: 5,
                    },
                }),
            },
            TestCase {
                // TC2: Invalid InstrumentKind::Spot Candle Subscription w/ unsupported interval
                input: Subscription::new(
                    ExchangeId::Kraken,
                    ("xbt", "usd", InstrumentKind::Spot),
                    SubKind::Candle(Interval::Month3),
                ),
                expected: Err(SocketError::Unsupported {
                    entity: "kraken",
                    item: Interval::Month3.to_string(),
                }),
            },
        ];

        for (index, test) in cases.into_iter().enumerate() {
            let actual = Kraken::subscription(&test.input);
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
    fn test_kraken_transform() {
        let mut transformer = kraken(vec![
            Subscription::from((
                ExchangeId::Kraken,
                "XBT",
                "USD",
                InstrumentKind::Spot,
                SubKind::Trade,
            )),
            Subscription::from((
                ExchangeId::Kraken,
                "XBT",
                "USD",
                InstrumentKind::Spot,
                SubKind::Candle(Interval::Minute5),
            )),
        ]);

        let timestamp = Utc::now();

        struct TestCase {
            input: KrakenMessage,
            expected: Vec<Result<MarketEvent, SocketError>>,
        }

        let cases = vec![
            TestCase {
                // TC0: KrakenMessage Spot trades w/ known SubscriptionId
                input: KrakenMessage::Trades(KrakenTrades {
                    subscription_id: SubscriptionId::from("trade|XBT/USD"),
                    trades: vec![
                        KrakenTrade {
                            price: 1.0,
                            quantity: 1.0,
                            time: timestamp,
                            side: Side::Buy,
                        },
                        KrakenTrade {
                            price: 2.0,
                            quantity: 2.0,
                            time: timestamp,
                            side: Side::Sell,
                        },
                    ],
                }),
                expected: vec![
                    Ok(MarketEvent {
                        exchange_time: timestamp,
                        received_time: timestamp,
                        exchange: Exchange::from(ExchangeId::Kraken),
                        instrument: Instrument::from(("xbt", "usd", InstrumentKind::Spot)),
                        kind: DataKind::Trade(PublicTrade {
                            id: kraken_trade_id(timestamp, Side::Buy, 1.0, 1.0),
                            price: 1.0,
                            quantity: 1.0,
                            side: Side::Buy,
                        }),
                    }),
                    Ok(MarketEvent {
                        exchange_time: timestamp,
                        received_time: timestamp,
                        exchange: Exchange::from(ExchangeId::Kraken),
                        instrument: Instrument::from(("xbt", "usd", InstrumentKind::Spot)),
                        kind: DataKind::Trade(PublicTrade {
                            id: kraken_trade_id(timestamp, Side::Sell, 2.0, 2.0),
                            price: 2.0,
                            quantity: 2.0,
                            side: Side::Sell,
                        }),
                    }),
                ],
            },
            TestCase {
                // TC1: KrakenMessage Spot trades w/ unknown SubscriptionId
                input: KrakenMessage::Trades(KrakenTrades {
                    subscription_id: SubscriptionId::from("unknown"),
                    trades: vec![KrakenTrade {
                        price: 1.0,
                        quantity: 1.0,
                        time: timestamp,
                        side: Side::Buy,
                    }],
                }),
                expected: vec![Err(SocketError::Unidentifiable(SubscriptionId::from(
                    "unknown",
                )))],
            },
            TestCase {
                // TC2: KrakenMessage Spot candles-5 w/ known SubscriptionId
                input: KrakenMessage::Candle(KrakenCandle {
                    subscription_id: SubscriptionId::from("ohlc-5|XBT/USD"),
                    candle: KrakenCandleData {
                        start_time: timestamp
                            .checked_sub_signed(chrono::Duration::minutes(5))
                            .unwrap(),
                        end_time: timestamp,
                        open: 7000.70000,
                        high: 7000.70000,
                        low: 1000.60000,
                        close: 3586.60000,
                        volume: 0.03373000,
                        trade_count: 50000,
                    },
                }),
                expected: vec![Ok(MarketEvent {
                    exchange_time: timestamp,
                    received_time: timestamp,
                    exchange: Exchange::from(ExchangeId::Kraken),
                    instrument: Instrument::from(("xbt", "usd", InstrumentKind::Spot)),
                    kind: DataKind::Candle(Candle {
                        start_time: timestamp
                            .checked_sub_signed(chrono::Duration::minutes(5))
                            .unwrap(),
                        end_time: timestamp,
                        open: 7000.70000,
                        high: 7000.70000,
                        low: 1000.60000,
                        close: 3586.60000,
                        volume: 0.03373000,
                        trade_count: 50000,
                    }),
                })],
            },
            TestCase {
                // TC3: KrakenMessage Spot candles-5 w/ unknown SubscriptionId
                input: KrakenMessage::Candle(KrakenCandle {
                    subscription_id: SubscriptionId::from("unknown"),
                    candle: KrakenCandleData {
                        start_time: timestamp,
                        end_time: timestamp
                            .checked_add_signed(chrono::Duration::minutes(5))
                            .unwrap(),
                        open: 7000.70000,
                        high: 7000.70000,
                        low: 1000.60000,
                        close: 3586.60000,
                        volume: 0.03373000,
                        trade_count: 50000,
                    },
                }),
                expected: vec![Err(SocketError::Unidentifiable(SubscriptionId::from(
                    "unknown",
                )))],
            },
            TestCase {
                // TC4: KrakenMessage Heartbeat returns empty vector
                input: KrakenMessage::KrakenEvent(KrakenEvent::Heartbeat),
                expected: vec![],
            },
            TestCase {
                // TC5: KrakenMessage Error returns empty vector
                input: KrakenMessage::KrakenEvent(KrakenEvent::Error(KrakenError {
                    message: "error message".to_string(),
                })),
                expected: vec![Err(SocketError::Exchange("error message".to_string()))],
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
                            received_time: timestamp,
                            ..actual
                        };
                        assert_eq!(
                            actual, expected,
                            "TC{} failed to assert_eq Oks at vector index {}",
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
