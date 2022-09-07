use super::model::{BinanceMessage, BinanceSubResponse};
use crate::{
    model::{MarketEvent, SubKind},
    ExchangeId, ExchangeTransformer, Subscriber, Subscription, SubscriptionIds, SubscriptionMeta,
};
use barter_integration::{error::SocketError, model::SubscriptionId, protocol::websocket::WsMessage, Transformer, Validator};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;
use tokio::sync::mpsc;

/// `BinanceFuturesUsd` [`Subscriber`](crate::Subscriber) &
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
                // eg/ SubscriptionId("@depth@100ms|BTCUSDT")
                ids.insert(
                    BinanceFuturesUsd::subscription_id(channel, &market.to_uppercase()),
                    subscription.clone()
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
                        trade
                    )))],
                    Err(error) => vec![Err(error)]
                }
            }
            BinanceMessage::OrderBookL2Update(update) => {
                match self.ids.find_instrument(&update.subscription_id) {
                    Ok(instrument) => vec![Ok(MarketEvent::from((
                        BinanceFuturesUsd::EXCHANGE,
                        instrument,
                        update
                    )))],
                    Err(error) => vec![Err(error)]
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

    /// [`BinanceFuturesUsd`] L2 OrderBook channel name.
    ///
    /// See docs: <https://binance-docs.github.io/apidocs/futures/en/#diff-book-depth-streams>
    pub const CHANNEL_ORDER_BOOK_L2: &'static str = "@depth@100ms";

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
            SubKind::OrderBookL2 => Self::CHANNEL_ORDER_BOOK_L2,
            other => return Err(SocketError::Unsupported {
                    entity: BinanceFuturesUsd::EXCHANGE.as_str(),
                    item: other.to_string(),
                })
        };

        // Determine BinanceFuturesUsd market using the Instrument
        let market = format!("{}{}", sub.instrument.base, sub.instrument.quote);

        Ok((channel, market))
    }

    /// Build a [`BinanceFuturesUsd`] compatible [`SubscriptionId`] using the channel & market
    /// provided. This is used to associate [`BinanceFuturesUsd`] data structures received over
    /// the WebSocket with it's original Barter [`Subscription`].
    ///
    /// eg/ SubscriptionId("@depth@100ms|BTCUSDT")
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
    use chrono::Utc;
    use barter_integration::model::{Exchange, Instrument, InstrumentKind, Side};
    use crate::exchange::binance::model::{BinanceOrderBookL2Update, BinanceTrade};
    use crate::model::{DataKind, Interval, LevelDelta, OrderBookDelta, PublicTrade};
    use super::*;

    fn binance_futures_usd(subscriptions: Vec<Subscription>) -> BinanceFuturesUsd {
        let ids = SubscriptionIds(
            subscriptions
                .into_iter()
                .map(|sub| {
                    let subscription_id = match (&sub.kind, &sub.instrument.kind) {
                        (SubKind::Trade, InstrumentKind::FuturePerpetual) => BinanceFuturesUsd::subscription_id(
                            BinanceFuturesUsd::CHANNEL_TRADES,
                            &format!("{}{}", sub.instrument.base, sub.instrument.quote).to_uppercase(),
                        ),
                        (SubKind::OrderBookL2, InstrumentKind::FuturePerpetual ) => BinanceFuturesUsd::subscription_id(
                            BinanceFuturesUsd::CHANNEL_ORDER_BOOK_L2,
                            &format!("{}{}", sub.instrument.base, sub.instrument.quote).to_uppercase(),
                        ),
                        (_, _) => {
                            panic!("not supported")
                        }
                    };

                    (subscription_id, sub)
                })
                .collect()
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
                // TC2: Supported InstrumentKind::FuturePerpetual OrderBookL2 subscription
                input: Subscription::new(
                    ExchangeId::BinanceFuturesUsd,
                    ("btc", "usdt", InstrumentKind::FuturePerpetual),
                    SubKind::OrderBookL2,
                ),
                expected: Ok(("@depth@100ms", "btcusdt".to_owned())),
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
                SubKind::OrderBookL2,
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
                    side: Side::Buy
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
                    side: Side::Buy
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
                        side: Side::Buy
                    })
                })]
            },
            TestCase {
                // TC2: BinanceMessage FuturePerpetual OrderBookL2 w/ known SubscriptionId
                input: BinanceMessage::OrderBookL2Update(BinanceOrderBookL2Update {
                    subscription_id: SubscriptionId::from("@depth@100ms|BTCUSDT"),
                    event_time: time,
                    transaction_time: time,
                    first_update_id: 100,
                    last_update_id: 110,
                    last_event_last_update_id: 99,
                    bids: vec![
                        LevelDelta::from((1000.0, 1.0)),
                        LevelDelta::from((2000.0, 1.0)),
                        LevelDelta::from((3000.0, 1.0)),
                    ],
                    asks: vec![
                        LevelDelta::from((4000.0, 1.0)),
                        LevelDelta::from((5000.0, 1.0)),
                        LevelDelta::from((6000.0, 1.0)),
                    ]
                }),
                expected: vec![Ok(MarketEvent {
                    exchange_time: time,
                    received_time: time,
                    exchange: Exchange::from(ExchangeId::BinanceFuturesUsd),
                    instrument: Instrument::from(("btc", "usdt", InstrumentKind::FuturePerpetual)),
                    kind: DataKind::OrderBookDelta(OrderBookDelta {
                        bid_deltas: vec![
                            LevelDelta::from((1000.0, 1.0)),
                            LevelDelta::from((2000.0, 1.0)),
                            LevelDelta::from((3000.0, 1.0)),
                        ],
                        ask_deltas: vec![
                            LevelDelta::from((4000.0, 1.0)),
                            LevelDelta::from((5000.0, 1.0)),
                            LevelDelta::from((6000.0, 1.0)),
                        ]
                    })
                })]
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