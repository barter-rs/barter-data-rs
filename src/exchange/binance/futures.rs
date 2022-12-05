use super::model::{BinanceMessage, BinanceSubResponse};
use crate::{
    model::{subscription::SubKind, MarketEvent},
    ExchangeId, ExchangeTransformer, Subscriber, Subscription, SubscriptionIds, SubscriptionMeta,
};
use barter_integration::{
    error::SocketError, model::SubscriptionId, protocol::websocket::WsMessage, Transformer,
    Validator,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;
use tokio::sync::mpsc;

/// [`BinanceFuturesUsd`] [`Subscriber`](crate::Subscriber) &
/// [`ExchangeTransformer`](crate::ExchangeTransformer) implementor for the collection
/// of `Futures` data.
#[derive(Clone, Eq, PartialEq, Debug, Deserialize, Serialize)]
pub struct BinanceFuturesUsd {
    pub ids: SubscriptionIds,
}

impl Subscriber for BinanceFuturesUsd {
    type SubResponse = BinanceSubResponse;

    fn base_url() -> &'static str {
        "wss://fstream.binance.com/ws"
    }

    fn build_subscription_meta(
        subscriptions: &[Subscription],
    ) -> Result<SubscriptionMeta, SocketError> {
        // Allocate SubscriptionIds HashMap to track identifiers for each actioned Subscription
        let mut ids = SubscriptionIds(HashMap::with_capacity(subscriptions.len()));

        // Map Barter Subscriptions to BinanceFuturesUsd 'StreamNames'
        let stream_names = subscriptions
            .iter()
            .map(|subscription| {
                // Determine BinanceFuturesUsd specific channel & market for this Barter Subscription
                let (channel, market) = Self::build_channel_meta(subscription)?;

                // Use "channel|market" as the SubscriptionId key in the SubscriptionIds
                // '--> Uppercase market to match incoming exchange event
                // eg/ SubscriptionId("@aggTrade|BTCUSDT")
                ids.insert(
                    BinanceFuturesUsd::subscription_id(channel, &market.to_uppercase()),
                    subscription.clone(),
                );

                // Construct BinanceFuturesUsd 'StreamName' eg/ "btcusdt@aggTrade"
                // '--> Lowercase market because the subscription 'StreamName' must be lowercase
                Ok(format!("{market}{channel}"))
            })
            .collect::<Result<Vec<_>, SocketError>>()?;

        // Use channels to construct a Binance subscription WsMessage
        let subscriptions = Self::subscriptions(stream_names);

        Ok(SubscriptionMeta {
            ids,
            expected_responses: subscriptions.len(),
            subscriptions,
        })
    }
}

impl ExchangeTransformer for BinanceFuturesUsd {
    const EXCHANGE: ExchangeId = ExchangeId::BinanceFuturesUsd;
    fn new(_: mpsc::UnboundedSender<WsMessage>, ids: SubscriptionIds) -> Self {
        Self { ids }
    }
}

impl Transformer<MarketEvent> for BinanceFuturesUsd {
    type Input = BinanceMessage;
    type OutputIter = Vec<Result<MarketEvent, SocketError>>;

    fn transform(&mut self, input: Self::Input) -> Self::OutputIter {
        match input {
            BinanceMessage::Trade(trade) => {
                match self.ids.find_instrument(&trade.subscription_id) {
                    Ok(instrument) => vec![Ok(MarketEvent::from((
                        BinanceFuturesUsd::EXCHANGE,
                        instrument,
                        trade,
                    )))],
                    Err(error) => vec![Err(error)],
                }
            }
            BinanceMessage::OrderBookSnapshot(snapshot) => {
                match self.ids.find_instrument(&snapshot.subscription_id) {
                    Ok(instrument) => vec![Ok(MarketEvent::from((
                        BinanceFuturesUsd::EXCHANGE,
                        instrument,
                        snapshot,
                    )))],
                    Err(error) => vec![Err(error)],
                }
            }
            BinanceMessage::Liquidation(liquidation) => {
                match self.ids.find_instrument(&liquidation.order.subscription_id) {
                    Ok(instrument) => vec![Ok(MarketEvent::from((
                        BinanceFuturesUsd::EXCHANGE,
                        instrument,
                        liquidation,
                    )))],
                    Err(error) => vec![Err(error)],
                }
            }
        }
    }
}

impl BinanceFuturesUsd {
    /// [`BinanceFuturesUsd`] aggregated trades channel name.
    ///
    /// See docs: <https://binance-docs.github.io/apidocs/futures/en/#aggregate-trade-streams>
    pub const CHANNEL_TRADES: &'static str = "@aggTrade";

    /// [`BinanceFuturesUsd`] OrderBook channel name. Note that currently additional channel
    /// information for for OrderBook latency (100ms) and depth (20 levels) is included.
    ///
    /// See docs: <https://binance-docs.github.io/apidocs/futures/en/#partial-book-depth-streams>
    pub const CHANNEL_ORDER_BOOK: &'static str = "@depth20@100ms";

    /// [`BinanceFuturesUsd`] liquidation orders channel name
    ///
    /// See docs: <https://binance-docs.github.io/apidocs/futures/en/#liquidation-order-streams>
    pub const CHANNEL_LIQUIDATIONS: &'static str = "@forceOrder";

    /// Determine the [`BinanceFuturesUsd`] channel metadata associated with an input
    /// Barter [`Subscription`]. This includes the [`BinanceFuturesUsd`] `&str` channel
    /// identifier, and a `String` market identifier. Both are used to build a
    /// [`BinanceFuturesUsd`] subscription payload.
    ///
    /// Example Ok return: Ok("@aggTrade", "btcusdt")
    /// where channel == "@aggTrade" & market == "btcusdt"
    pub fn build_channel_meta(sub: &Subscription) -> Result<(&str, String), SocketError> {
        // Validate provided Subscription InstrumentKind is supported by BinanceFuturesUsd
        let sub = sub.validate()?;

        // Determine the BinanceFuturesUsd channel
        let channel = match &sub.kind {
            SubKind::Trade => Self::CHANNEL_TRADES,
            SubKind::OrderBook => Self::CHANNEL_ORDER_BOOK,
            SubKind::Liquidation => Self::CHANNEL_LIQUIDATIONS,
            other => {
                return Err(SocketError::Unsupported {
                    entity: BinanceFuturesUsd::EXCHANGE.as_str(),
                    item: other.to_string(),
                })
            }
        };

        // Determine BinanceFuturesUsd market using the Instrument
        let market = format!("{}{}", sub.instrument.base, sub.instrument.quote);

        Ok((channel, market))
    }

    /// Build a [`BinanceFuturesUsd`] compatible [`SubscriptionId`] using the channel & market
    /// provided. This is used to associate [`BinanceFuturesUsd`] data structures received over
    /// the WebSocket with it's original Barter [`Subscription`].
    ///
    /// eg/ SubscriptionId("@aggTrade|BTCUSDT")
    pub fn subscription_id(channel: &str, market: &str) -> SubscriptionId {
        SubscriptionId::from(format!("{channel}|{market}"))
    }

    /// Build a [`BinanceFuturesUsd`] compatible subscription message using the
    /// 'StreamNames' provided.
    pub fn subscriptions(stream_names: Vec<String>) -> Vec<WsMessage> {
        vec![WsMessage::Text(
            json!({
                "method": "SUBSCRIBE",
                "params": stream_names,
                "id": 1
            })
            .to_string(),
        )]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::exchange::binance::model::{
        BinanceLiquidation, BinanceLiquidationOrder, BinanceTrade,
    };
    use crate::model::Liquidation;
    use crate::model::{subscription::Interval, DataKind, PublicTrade};
    use barter_integration::model::{Exchange, Instrument, InstrumentKind, Side};
    use chrono::Utc;

    fn binance_futures_usd(subscriptions: Vec<Subscription>) -> BinanceFuturesUsd {
        let ids = SubscriptionIds(
            subscriptions
                .into_iter()
                .map(|sub| {
                    let subscription_id = match (&sub.kind, &sub.instrument.kind) {
                        (SubKind::Trade, InstrumentKind::FuturePerpetual) => {
                            BinanceFuturesUsd::subscription_id(
                                BinanceFuturesUsd::CHANNEL_TRADES,
                                &format!("{}{}", sub.instrument.base, sub.instrument.quote)
                                    .to_uppercase(),
                            )
                        }
                        (SubKind::Liquidation, InstrumentKind::FuturePerpetual) => {
                            BinanceFuturesUsd::subscription_id(
                                BinanceFuturesUsd::CHANNEL_LIQUIDATIONS,
                                &format!("{}{}", sub.instrument.base, sub.instrument.quote)
                                    .to_uppercase(),
                            )
                        }
                        (_, _) => {
                            panic!("not supported")
                        }
                    };

                    (subscription_id, sub)
                })
                .collect(),
        );

        BinanceFuturesUsd { ids }
    }

    #[test]
    fn test_build_channel_meta() {
        struct TestCase<'a> {
            input: Subscription,
            expected: Result<(&'a str, String), SocketError>,
        }

        let cases = vec![
            TestCase {
                // TC0: Unsupported InstrumentKind::Spot subscription
                input: Subscription::new(
                    ExchangeId::BinanceFuturesUsd,
                    ("btc", "usdt", InstrumentKind::Spot),
                    SubKind::Trade,
                ),
                expected: Err(SocketError::Unsupported {
                    entity: "",
                    item: "".to_string(),
                }),
            },
            TestCase {
                // TC1: Supported InstrumentKind::FuturePerpetual trades subscription
                input: Subscription::new(
                    ExchangeId::BinanceFuturesUsd,
                    ("btc", "usdt", InstrumentKind::FuturePerpetual),
                    SubKind::Trade,
                ),
                expected: Ok(("@aggTrade", "btcusdt".to_owned())),
            },
            TestCase {
                // TC2: Unsupported InstrumentKind::FuturePerpetual OrderBookL2Delta subscription
                input: Subscription::new(
                    ExchangeId::BinanceFuturesUsd,
                    ("btc", "usdt", InstrumentKind::FuturePerpetual),
                    SubKind::OrderBookL2Delta,
                ),
                expected: Err(SocketError::Unsupported {
                    entity: "",
                    item: "".to_string(),
                }),
            },
            TestCase {
                // TC3: Unsupported InstrumentKind::FuturePerpetual candle subscription
                input: Subscription::new(
                    ExchangeId::BinanceFuturesUsd,
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
            let actual = BinanceFuturesUsd::build_channel_meta(&test.input);
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
    fn test_binance_transform() {
        let mut transformer = binance_futures_usd(vec![
            Subscription::from((
                ExchangeId::BinanceFuturesUsd,
                "btc",
                "usdt",
                InstrumentKind::FuturePerpetual,
                SubKind::Trade,
            )),
            Subscription::from((
                ExchangeId::BinanceFuturesUsd,
                "btc",
                "usdt",
                InstrumentKind::FuturePerpetual,
                SubKind::Liquidation,
            )),
        ]);

        let time = Utc::now();

        struct TestCase {
            input: BinanceMessage,
            expected: Vec<Result<MarketEvent, SocketError>>,
        }

        let cases = vec![
            TestCase {
                // TC0: BinanceMessage with unknown SubscriptionId
                input: BinanceMessage::Trade(BinanceTrade {
                    subscription_id: SubscriptionId::from("unknown"),
                    time,
                    id: 0,
                    price: 1000.0,
                    quantity: 1.0,
                    side: Side::Buy,
                }),
                expected: vec![Err(SocketError::Unidentifiable(SubscriptionId::from(
                    "unknown",
                )))],
            },
            TestCase {
                // TC1: BinanceMessage FuturePerpetual trade w/ known SubscriptionId
                input: BinanceMessage::Trade(BinanceTrade {
                    subscription_id: SubscriptionId::from("@aggTrade|BTCUSDT"),
                    time,
                    id: 0,
                    price: 1000.0,
                    quantity: 1.0,
                    side: Side::Buy,
                }),
                expected: vec![Ok(MarketEvent {
                    exchange_time: time,
                    received_time: time,
                    exchange: Exchange::from(ExchangeId::BinanceFuturesUsd),
                    instrument: Instrument::from(("btc", "usdt", InstrumentKind::FuturePerpetual)),
                    kind: DataKind::Trade(PublicTrade {
                        id: "0".to_string(),
                        price: 1000.0,
                        quantity: 1.0,
                        side: Side::Buy,
                        sequence: None,
                    }),
                })],
            },
            TestCase {
                // TC2: BinanceMessage FuturePerpetual liquidation
                input: BinanceMessage::Liquidation(BinanceLiquidation {
                    order: BinanceLiquidationOrder {
                        subscription_id: SubscriptionId::from("@forceOrder|BTCUSDT"),
                        side: Side::Buy,
                        price: 1.0,
                        quantity: 20.0,
                        time,
                    },
                }),
                expected: vec![Ok(MarketEvent {
                    exchange_time: time,
                    received_time: time,
                    exchange: Exchange::from(ExchangeId::BinanceFuturesUsd),
                    instrument: Instrument::from(("btc", "usdt", InstrumentKind::FuturePerpetual)),
                    kind: DataKind::Liquidation(Liquidation {
                        side: Side::Buy,
                        price: 1.0,
                        quantity: 20.0,
                        time,
                    }),
                })],
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
