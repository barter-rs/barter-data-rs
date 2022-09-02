use crate::{
    model::SubKind, ExchangeId, ExchangeTransformer, MarketEvent, Subscriber, Subscription,
    SubscriptionIds, SubscriptionMeta,
};
use barter_integration::{
    error::SocketError, model::SubscriptionId, protocol::websocket::WsMessage, Transformer,
};
use model::{KrakenEvent, KrakenMessage, KrakenSubResponse};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;
use tokio::sync::mpsc;
use tracing::debug;

/// [`Kraken`] specific data structures.
mod model;

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
                // Determine the Kraken specific channel & market for this Barter Subscription
                let (channel, market) = Self::get_channel_meta(subscription)?;

                // Construct Kraken specific subscription message
                let kraken_subscription = Self::subscription(channel, &market);

                // Use "channel|market" as the SubscriptionId key in SubscriptionIds HashMap
                ids.insert(
                    Kraken::subscription_id(channel, market),
                    subscription.clone(),
                );

                Ok(kraken_subscription)
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
        let trades = match input {
            KrakenMessage::Trades(trades) => trades,
            KrakenMessage::KrakenEvent(KrakenEvent::Heartbeat) => {
                debug!(exchange_id = %Kraken::EXCHANGE, "received heartbeat");
                return vec![];
            }
            KrakenMessage::KrakenEvent(KrakenEvent::Error(error)) => {
                return vec![Err(SocketError::Exchange(error.message))]
            }
        };

        let instrument = match self.ids.find_instrument(trades.subscription_id) {
            Ok(instrument) => instrument,
            Err(error) => return vec![Err(error)],
        };

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
}

impl Kraken {
    /// Determine the `Kraken` channel metadata associated with an input Barter [`Subscription`].
    /// This includes the `Kraken` &str channel, and a `String` market identifier. Both are used to
    /// build an `Kraken` subscription payload, as well as the associated [`SubscriptionId`] via
    /// [`Self::subscription_id`].
    ///
    /// Example Ok Return: Ok("trade", "XBT/USD")
    /// where channel == "trade" & market == "XBT/USD".
    fn get_channel_meta(sub: &Subscription) -> Result<(&str, String), SocketError> {
        // Determine Kraken channel using the Subscription SubKind
        let channel = match &sub.kind {
            SubKind::Trade => "trade",
            other => {
                return Err(SocketError::Unsupported {
                    entity: Self::EXCHANGE.as_str(),
                    item: other.to_string(),
                })
            }
        };

        // Determine Kraken market using the Instrument
        let market = format!("{}/{}", sub.instrument.base, sub.instrument.quote).to_uppercase();

        Ok((channel, market))
    }

    /// Build a `Kraken` compatible [`SubscriptionId`] using the channel & market provided. This is
    /// used to associate `Kraken` data structures received over the WebSocket with their original
    /// Barter [`Subscription`].
    fn subscription_id(channel: &str, market: String) -> SubscriptionId {
        SubscriptionId::from(format!("{channel}|{market}"))
    }

    /// Build a `Kraken` compatible subscription message using the channel & market provided.
    fn subscription(channel: &str, market: &str) -> WsMessage {
        WsMessage::Text(
            json!({
                "event": "subscribe",
                "pair": [market],
                "subscription": {
                    "name": channel
                }
            })
            .to_string(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::exchange::kraken::model::{KrakenError, KrakenTrade, KrakenTrades};
    use crate::model::{DataKind, PublicTrade};
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

                    let subscription_id = match &sub.kind {
                        SubKind::Trade => {
                            format!(
                                "trade|{}",
                                format!("{}/{}", sub.instrument.base, sub.instrument.quote)
                                    .to_uppercase()
                            )
                        }
                        _ => panic!("non trade Subscriptions not implemented yet"),
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
    fn test_get_channel_meta() {
        struct TestCase {
            input: Subscription,
            expected: Result<(&'static str, String), SocketError>,
        }

        let cases = vec![TestCase {
            // TC0: InstrumentKind::Spot is supported
            input: Subscription::new(
                ExchangeId::Kraken,
                ("xbt", "USD", InstrumentKind::Spot),
                SubKind::Trade,
            ),
            expected: Ok(("trade", "XBT/USD".to_owned())),
        }];

        for (index, test) in cases.into_iter().enumerate() {
            let actual = Kraken::get_channel_meta(&test.input);
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
        let mut transformer = kraken(vec![Subscription::from((
            ExchangeId::Kraken,
            "XBT",
            "USD",
            InstrumentKind::Spot,
            SubKind::Trade,
        ))]);

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
                // TC2: KrakenMessage Heartbeat returns empty vector
                input: KrakenMessage::KrakenEvent(KrakenEvent::Heartbeat),
                expected: vec![],
            },
            TestCase {
                // TC3: KrakenMessage Error returns empty vector
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
                "TestCase {} failed on vector length check",
                index
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
