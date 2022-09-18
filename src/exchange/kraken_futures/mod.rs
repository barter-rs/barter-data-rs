use crate::{
    ExchangeId, ExchangeTransformer, MarketEvent, Subscriber, Subscription, SubscriptionIds,
    SubscriptionMeta,
};
use barter_integration::{
    error::SocketError, model::SubscriptionId, protocol::websocket::WsMessage, Transformer,
};
use model::{
    KrakenFuturesUsdMessage, KrakenFuturesUsdSubKind, KrakenFuturesUsdSubResponse,
    KrakenFuturesUsdSubscription,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tokio::sync::mpsc;

/// [`KrakenFuturesUsd`] specific data structures.
pub mod model;

/// [`KrakenFuturesUsd`] [`Subscriber`] & [`ExchangeTransformer`] implementor for
/// the collection of `Spot` data.
#[derive(Clone, Eq, PartialEq, Debug, Deserialize, Serialize)]
pub struct KrakenFuturesUsd {
    pub ids: SubscriptionIds,
}

impl Subscriber for KrakenFuturesUsd {
    type SubResponse = KrakenFuturesUsdSubResponse;

    fn base_url() -> &'static str {
        "wss://futures.kraken.com/ws/v1"
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
                let kraken_subscription = KrakenFuturesUsd::subscription(subscription)?;

                // Determine the SubscriptionId ("{channel}|{market} ")for this KrakenSubscription
                // eg/ SubscriptionId("ohlc-5|XBT/USD")
                let subscription_id = SubscriptionId::from(&kraken_subscription);

                // Insert SubscriptionId to Barter Subscription Entry in SubscriptionIds HashMap
                ids.insert(subscription_id, subscription.clone());

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

impl ExchangeTransformer for KrakenFuturesUsd {
    const EXCHANGE: ExchangeId = ExchangeId::KrakenFuturesUsd;

    fn new(_: mpsc::UnboundedSender<WsMessage>, ids: SubscriptionIds) -> Self {
        Self { ids }
    }
}

impl Transformer<MarketEvent> for KrakenFuturesUsd {
    type Input = KrakenFuturesUsdMessage;
    type OutputIter = Vec<Result<MarketEvent, SocketError>>;

    fn transform(&mut self, input: Self::Input) -> Self::OutputIter {
        match input {
            KrakenFuturesUsdMessage::Trade(trade) => {
                let instrument = match self.ids.find_instrument(trade.product_id.clone()) {
                    Ok(instrument) => instrument,
                    Err(error) => {
                        return vec![Err(error)];
                    }
                };
                vec![Ok(MarketEvent::from((
                    KrakenFuturesUsd::EXCHANGE,
                    instrument.clone(),
                    trade,
                )))]
            }
            // TradeSnapshot consists of old trades, so are ignored
            KrakenFuturesUsdMessage::TradeSnapshot { .. } => {
                vec![]
                /*
                let instrument = match self.ids.find_instrument(product_id.clone()) {
                    Ok(instrument) => instrument,
                    Err(error) => {
                        return vec![Err(error)];
                    }
                };

                trades
                    .into_iter()
                    .map(|trade| {
                        Ok(MarketEvent::from((
                            KrakenFuturesUsd::EXCHANGE,
                            instrument.clone(),
                            trade,
                        )))
                    })
                    .collect()
                */
            }
        }
    }
}

impl KrakenFuturesUsd {
    /// Translate a Barter [`Subscription`] into a [`KrakenFuturesUsd`] compatible subscription message.
    fn subscription(sub: &Subscription) -> Result<KrakenFuturesUsdSubscription, SocketError> {
        // Determine Kraken pair using the Instrument
        let pair = format!("PI_{}{}", sub.instrument.base, sub.instrument.quote).to_uppercase();

        // Determine the KrakenSubKind from the Barter SubKind
        let kind = KrakenFuturesUsdSubKind::try_from(&sub.kind)?;

        Ok(KrakenFuturesUsdSubscription::new(pair, kind))
    }
}

#[cfg(test)]
mod tests {
    use super::model::KrakenFuturesUsdTrade;
    use super::*;
    use crate::exchange::datetime_utc_from_epoch_duration;
    use crate::model::{DataKind, PublicTrade, SubKind};
    use barter_integration::model::{Exchange, Instrument, InstrumentKind, Side};
    use chrono::Utc;
    use std::time::Duration;

    fn kraken_futures(subscriptions: Vec<Subscription>) -> KrakenFuturesUsd {
        let ids = SubscriptionIds(
            subscriptions
                .into_iter()
                .map(|sub| {
                    let subscription_id = match (&sub.kind, &sub.instrument.kind) {
                        (SubKind::Trade, InstrumentKind::FuturePerpetual) => {
                            let product_id =
                                format!("PI_{}{}", sub.instrument.base, sub.instrument.quote)
                                    .to_uppercase();
                            format!("{}", product_id)
                        }
                        (_, _) => {
                            panic!("not supported")
                        }
                    };

                    (SubscriptionId::from(subscription_id), sub)
                })
                .collect(),
        );

        KrakenFuturesUsd { ids }
    }

    #[test]
    fn test_build_kraken_futures_subscription() {
        struct TestCase {
            input: Subscription,
            expected: Result<KrakenFuturesUsdSubscription, SocketError>,
        }

        let cases = vec![TestCase {
            // TC0: Supported InstrumentKind::FuturePerpetual trades subscription
            input: Subscription::new(
                ExchangeId::KrakenFuturesUsd,
                ("XBT", "USD", InstrumentKind::FuturePerpetual),
                SubKind::Trade,
            ),
            expected: Ok(KrakenFuturesUsdSubscription::new(
                "PI_XBTUSD".to_owned(),
                KrakenFuturesUsdSubKind::Trade("trade"),
            )),
        }];

        for (index, test) in cases.into_iter().enumerate() {
            let actual = KrakenFuturesUsd::subscription(&test.input);
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
    fn test_kraken_futures_transform() {
        let mut transformer = kraken_futures(vec![Subscription::from((
            ExchangeId::KrakenFuturesUsd,
            "xbt",
            "usd",
            InstrumentKind::FuturePerpetual,
            SubKind::Trade,
        ))]);

        let time = Utc::now();

        struct TestCase {
            input: KrakenFuturesUsdMessage,
            expected: Vec<Result<MarketEvent, SocketError>>,
        }

        let cases = vec![
            TestCase {
                // TC0: KrakenFuturesUsd Futures trades w/ known SubscriptionId
                input: KrakenFuturesUsdMessage::Trade(KrakenFuturesUsdTrade {
                    product_id: SubscriptionId::from("PI_XBTUSD"),
                    uid: "test_id".to_owned(),
                    side: Side::Buy,
                    seq: 14,
                    time: 1523436,
                    quantity: 1.0,
                    price: 2435.0,
                }),
                expected: vec![Ok(MarketEvent {
                    exchange_time: datetime_utc_from_epoch_duration(Duration::new(
                        1523436 / 1000,
                        1523436 % 1000 * 1000000,
                    )),
                    received_time: time,
                    exchange: Exchange::from(ExchangeId::KrakenFuturesUsd),
                    instrument: Instrument::from(("xbt", "usd", InstrumentKind::FuturePerpetual)),
                    kind: DataKind::Trade(PublicTrade {
                        id: "test_id".to_string(),
                        price: 2435.0,
                        quantity: 1.0,
                        side: Side::Buy,
                    }),
                })],
            },
            TestCase {
                // TC1: KrakenFuturesUsd with unknown SubscriptionId
                input: KrakenFuturesUsdMessage::Trade(KrakenFuturesUsdTrade {
                    product_id: SubscriptionId::from("unknown"),
                    uid: "1".to_owned(),
                    side: Side::Buy,
                    seq: 14,
                    time: 1523436,
                    quantity: 1.0,
                    price: 2435.0,
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
