//!
//! Websocket implementation for Bitfinex v2 websocket API.
//! 
//! The currently supported streams are:
//! - [`Trades`](crate::model::subscription::SubKind::Trade): Aggregated trades.
//! - [`Candles`](crate::model::subscription::SubKind::Candle): OHLC candles.
//!
//! ## Notes:
//! ### SubscriptionId
//! - Successful Bitfinex subscription responses contain a numeric `CHANNEL_ID` that must be used to
//!   identify future messages relating to that subscription (not persistent across connections).
//! - To identify the initial subscription response containing the `CHANNEL_ID`, the "channel" &
//!   "market" identifiers can be used for the `SubscriptionId(channel|market)`
//!   (eg/ SubscriptionId("trades|tBTCUSD")).
//! - Once the subscription has been validated and the `CHANNEL_ID` determined, each `SubscriptionId`
//!   in the `SubscriptionIds` `HashMap` is mutated to become `SubscriptionId(CHANNEL_ID)`.
//!   eg/ SubscriptionId("trades|tBTCUSD") -> SubscriptionId(69)
//!
//! ### Connection Limits
//! - The user is allowed up to 20 connections per minute on the public API.
//! - Each connection can be used to connect up to 25 different channels.
//!
//! ### Trade Variants
//! - `Bitfinex` trades subscriptions results in receiving tag="te" & tag="tu" trades.
//! - Both appear to be identical payloads, but "te" arriving marginally faster.
//! - Therefore, tag="tu" trades are filtered out and considered only as additional Heartbeats.
//!
//! ### Candle Socket Quirk
//! The Candle socket sends multiple messages with the same timestamp. Most of the time the new message is
//! an update of the current timestamp, that is one of low, high, or close will have changed. Sometimes there
//! is no change if there has been no activity for a while.

use self::model::{
    BitfinexMessage, BitfinexPayload, BitfinexPlatformStatus, BitfinexSubResponse,
    BitfinexSubResponseKind,
};
use crate::{
    model::{
        MarketEvent,
        subscription::{Subscription, SubKind, SubscriptionIds, SubscriptionMeta, Interval},
    },
    ExchangeId, ExchangeTransformer, Subscriber
};
use async_trait::async_trait;
use barter_integration::{
    error::SocketError,
    model::{InstrumentKind, SubscriptionId},
    protocol::websocket::{WebSocket, WsMessage},
    Transformer, Validator,
};
use chrono::{DateTime, Duration, Utc};
use futures::StreamExt;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;
use tokio::sync::mpsc;
use tracing::debug;

/// [`Bitfinex`] specific data structures.
pub mod model;

/// The Barter supported websocket subscription channels for [`Bitfinex`]. 
/// 
/// See docs: <https://docs.bitfinex.com/docs/ws-public>.
#[derive(Debug, Clone, Copy, PartialEq)]
 enum BitfinexChannel { 
    /// Aggregated trades channel.
    Trades,
    /// OHLC candles channel.
    Candles,
}

impl BitfinexChannel {
    fn as_str(&self) -> &'static str {
        match &self {
            BitfinexChannel::Trades => "trades",
            BitfinexChannel::Candles => "candles",
        }
    }
}

impl ToString for BitfinexChannel {
    fn to_string(&self) -> String {
        match &self {
            BitfinexChannel::Trades => String::from("trades"),
            BitfinexChannel::Candles => String::from("candles"),
        }
    }
}

/// `Bitfinex` [`Subscriber`] & [`ExchangeTransformer`] implementor for the collection
/// of `Spot` data.
#[derive(Clone, Eq, PartialEq, Debug, Deserialize, Serialize)]
pub struct Bitfinex {
    /// Mapping of exchange-specific subscription Id to the [`Subscription`].
    pub ids: SubscriptionIds,
    /// Mapping of candle channel to most recently received timestamp. Used to deduplicate candle stream.
    candles_stamps: HashMap<u32, DateTime<Utc>>,
}

#[async_trait]
impl Subscriber for Bitfinex {
    type SubResponse = BitfinexSubResponse;

    fn base_url() -> &'static str {
        "wss://api-pub.bitfinex.com/ws/2"
    }

    fn build_subscription_meta(
        subscriptions: &[Subscription],
    ) -> Result<SubscriptionMeta, SocketError> {
        // Allocate SubscriptionIds HashMap to track identifiers for each actioned Subscription
        let mut ids = SubscriptionIds(HashMap::with_capacity(subscriptions.len()));

        // Map Barter subscriptions to Bitfinex subscriptions
        let subscriptions = subscriptions
            .iter()
            .map(|subscription| {
                // Determine the Bitfinex specific channel & market for this subscription
                let (channel, market) = Self::build_channel_meta(subscription)?;

                // Use "channel|market" as the Subscription key in the SubscriptionIds
                // eg/ SubscriptionId("trades|BTC/USDT")
                // '--> later switched to SubscriptionId(CHANNEL_ID) during subscription validation
                ids.insert(
                    Bitfinex::subscription_id(channel.as_str(), &market),
                    subscription.clone(),
                );

                // Construct Bitfinex specific subscription message
                Ok(Self::build_subscription_message(channel, &market))
            })
            .collect::<Result<Vec<_>, SocketError>>()?;

        Ok(SubscriptionMeta {
            ids,
            expected_responses: subscriptions.len(),
            subscriptions,
        })
    }

    async fn validate(
        mut ids: SubscriptionIds,
        websocket: &mut WebSocket,
        expected_responses: usize,
    ) -> Result<SubscriptionIds, SocketError> {
        // Establish time limit in which we expect to validate all the Subscriptions
        let timeout = Self::subscription_timeout();

        // Parameter to keep track of successful Subscription outcomes
        let mut num_success_responses = 0usize;
        let mut init_snapshots_received = 0usize;

        loop {
            // Break if all Subscriptions were a success
            if num_success_responses == expected_responses
                && init_snapshots_received == expected_responses
            {
                break Ok(ids);
            }

            tokio::select! {
                // If timeout reached, return SubscribeError
                _ = tokio::time::sleep(timeout) => {
                    break Err(SocketError::Subscribe(
                        format!("subscription validation timeout reached: {:?}", timeout))
                    )
                },

                // Parse incoming messages and determine subscription outcomes
                message = websocket.next() => match message {
                    Some(Ok(WsMessage::Text(payload))) => {
                        if let Ok(bitfinex_status) = serde_json::from_str::<BitfinexPlatformStatus>(&payload) {
                            bitfinex_status.validate()?;
                        }

                        else if let Ok(response) = serde_json::from_str::<Self::SubResponse>(&payload) {
                            // Validate SubResponse & identify Bitfinex CHANNEL_ID for this session
                            let (subscription_id, channel_id) = match response.validate() {

                                // Trade Subscription success
                                Ok(Self::SubResponse::Subscribed(BitfinexSubResponseKind::Trades {
                                    market, channel_id
                                })) => {
                                    (
                                        Bitfinex::subscription_id(Bitfinex::CHANNEL_TRADES, &market),
                                        channel_id
                                    )
                                },

                                // Candle Subscription success
                                Ok(Self::SubResponse::Subscribed(BitfinexSubResponseKind::Candles {
                                    channel_id, key,
                                })) => {
                                    (
                                        Bitfinex::subscription_id(Bitfinex::CHANNEL_CANDLES, &key),
                                        channel_id
                                    )
                                },

                                // Subscription failure
                                Err(err) => break Err(err),

                                // Subscription failure - not reachable after validate()
                                Ok(Self::SubResponse::Error(_)) => unreachable!()
                            };

                            // Replace SubscriptionId(market) with SubscriptionId(CHANNEL_ID)
                            if let Some(subscription) = ids.remove(&subscription_id) {
                                num_success_responses += 1;
                                ids.insert(SubscriptionId(channel_id.to_string()), subscription);
                            }
                        }

                        else {
                            // Already active Subscriptions will send initial snapshots
                            init_snapshots_received += 1;
                            continue;
                        }
                    },
                    Some(Ok(WsMessage::Close(close_frame))) => {
                        break Err(SocketError::Subscribe(format!(
                            "received WebSocket CloseFrame: {:?}", close_frame
                        )))
                    },
                    _ => continue,
                }
            }
        }
    }
}

impl Bitfinex {
    /// [`Bitfinex`] trades channel name.
    ///
    /// See docs: <https://docs.bitfinex.com/reference/ws-public-trades>
    const CHANNEL_TRADES: &'static str = "trades";

    /// [`Bitfinex`] candles channel name.
    ///
    /// See docs: <https://docs.bitfinex.com/reference/ws-public-candles>
    const CHANNEL_CANDLES: &'static str = "candles";

    /// Build the initial [`Bitfinex`] compatible [`SubscriptionId`] using the channel & market
    /// provided. Note this is only used to confirm subscription responses. The [`SubscriptionId`]
    /// is then replaced with the numeric `CHANNEL_ID` [`Bitfinex`] uses as message identifiers
    /// (not persistent across connection).
    ///
    /// This `SubscriptionId(CHANNEL_ID)` is then used to
    /// associated [`Bitfinex`] data structures received over the Websocket with it's original
    /// Barter [`Subscription`].
    ///
    /// eg/ SubscriptionId("trades|tBTCUSD")
    fn subscription_id(channel: &str, market: &str) -> SubscriptionId {
        SubscriptionId::from(format!("{channel}|{market}"))
    }

    /// Determine the [`Bitfinex`] channel metadata associated with an input Barter
    /// [`Subscription`]. This includes the `Bitfinex` &str channel, and a `String` market
    /// identifier. Both are used to build a `Bitfinex` subscription payload.
    ///
    /// Example Ok Return: Ok(BitfinexChannel::Trades, "tBTCUSD")
    /// Example Ok Return: Ok(BitfinexChannel::Candles, "trade:1m:tBTCUSD")
    /// where channel == BitfinexChannel::Trades & market == "tBTCUSD".
    fn build_channel_meta(sub: &Subscription) -> Result<(BitfinexChannel, String), SocketError> {
        let sub = sub.validate()?;

        // Determine Bitfinex channel
        let channel = match &sub.kind {
            SubKind::Trade => BitfinexChannel::Trades,
            SubKind::Candle(_) => BitfinexChannel::Candles,
            other => {
                return Err(SocketError::Unsupported {
                    entity: Self::EXCHANGE.as_str(),
                    item: other.to_string(),
                })
            }
        };

        // Determine Bitfinex market using the Subscription Instrument
        let market = match &sub.instrument.kind {
            InstrumentKind::Spot => format!(
                "t{}{}",
                sub.instrument.base.to_string().to_uppercase(),
                sub.instrument.quote.to_string().to_uppercase()
            ),
            other => return Err(SocketError::Unsupported {
                entity: Self::EXCHANGE.as_str(),
                item: other.to_string(),
            }),
        };

        // Modify for a candle subscription
        let market = if let SubKind::Candle(interval) = &sub.kind {
            format!("trade:{}:{}", interval.to_string(), market)
        } else {
            market
        };

        Ok((channel, market))
    }

    /// Build a [`Bitfinex`] compatible subscription message using the channel & market provided.
    ///
    /// Example arguments: channel = "trades", market = "tBTCUSD"
    fn build_subscription_message(channel: BitfinexChannel, market: &str) -> WsMessage {
        match channel {
            BitfinexChannel::Trades => WsMessage::Text(
                json!({
                    "event": "subscribe",
                    "channel": Bitfinex::CHANNEL_TRADES,
                    "symbol": market,
                })
                .to_string(),
            ),
            BitfinexChannel::Candles => WsMessage::Text(
                json!({
                    "event": "subscribe",
                    "channel": Bitfinex::CHANNEL_CANDLES,
                    "key": market,
                })
                .to_string(),
            ),
        }
    }
}

impl ExchangeTransformer for Bitfinex {
    const EXCHANGE: ExchangeId = ExchangeId::Bitfinex;

    fn new(_: mpsc::UnboundedSender<WsMessage>, ids: SubscriptionIds) -> Self {
        Self {
            ids,
            candles_stamps: HashMap::new(),
        }
    }
}

impl Transformer<MarketEvent> for Bitfinex {
    type Input = BitfinexMessage;
    type OutputIter = Vec<Result<MarketEvent, SocketError>>;

    fn transform(&mut self, input: Self::Input) -> Self::OutputIter {
        let BitfinexMessage {
            channel_id,
            payload,
        } = input;

        match payload {
            BitfinexPayload::Heartbeat => {
                debug!(mapper = %Self::EXCHANGE, %channel_id, "received heartbeat");
                vec![]
            }
            BitfinexPayload::Trade(trade) => {
                match self
                    .ids
                    .find_instrument(&SubscriptionId(channel_id.to_string()))
                {
                    Ok(instrument) => vec![Ok(MarketEvent::from((
                        Bitfinex::EXCHANGE,
                        instrument,
                        trade,
                    )))],
                    Err(error) => vec![Err(error)],
                }
            }
            BitfinexPayload::Candle(candle) => {
                // Check validity of the new candle, as Bitfinex will send candles with outdated timestamps.
                // A current candle may be sent numerous times, with the understanding that each new message
                // is an update on the current candle (for example high or low has changed).
                if let Some(most_recent_time) = self.candles_stamps.get(&channel_id) {
                    // If the candle is outdated don't return data
                    if &candle.time < most_recent_time {
                        return vec![];
                    }
                    // If the time has advanced update latest timestamp
                    else if &candle.time > most_recent_time {
                        self.candles_stamps.insert(channel_id, candle.time.clone());
                    }
                }
                else {
                    // Make note of first timestamp
                    self.candles_stamps.insert(channel_id, candle.time.clone());
                }

                // Get the start time of this candle
                let sub = self
                    .ids
                    .get(&SubscriptionId(channel_id.to_string()))
                    .unwrap();
                let start_time = match sub.kind {
                    SubKind::Candle(interval) => {
                        match calculate_start_time(interval, &candle.time) {
                            Ok(start_time) => start_time,
                            Err(err) => return vec![Err(err)]
                        }
                    },
                    _ => return vec![Err(SocketError::Unidentifiable(SubscriptionId(channel_id.to_string())))],
                };

                // Determine Instrument associated with this Bitfinex Candle
                let instrument = match self.ids.find_instrument(&SubscriptionId(channel_id.to_string())) {
                    Ok(instrument) => instrument,
                    Err(error) => return vec![Err(error)],
                };

                vec![Ok(MarketEvent::from((
                    Bitfinex::EXCHANGE,
                    instrument,
                    candle,
                    start_time
                )))]
            }
        }
    }
}

fn calculate_start_time(interval: Interval, end_time: &DateTime<Utc>) -> Result<DateTime<Utc>, SocketError> {
    match interval {
        Interval::Minute1 => Ok(*end_time - Duration::minutes(1)),
        Interval::Minute5 => Ok(*end_time - Duration::minutes(5)),
        Interval::Minute15 => Ok(*end_time - Duration::minutes(15)),
        Interval::Minute30 => Ok(*end_time - Duration::minutes(30)),
        Interval::Hour1 => Ok(*end_time - Duration::hours(1)),
        Interval::Hour6 => Ok(*end_time - Duration::hours(6)),
        Interval::Hour12 => Ok(*end_time - Duration::hours(12)),
        Interval::Day1 => Ok(*end_time - Duration::days(1)),
        Interval::Week1 => Ok(*end_time - Duration::weeks(1)),
        Interval::Month1 => Ok(*end_time - Duration::weeks(4)),
        // Return error on unsupported intervals
        other => {
            return Err(SocketError::Unsupported {
                entity: Bitfinex::EXCHANGE.as_str(),
                item: other.to_string(),
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use barter_integration::{
        error::SocketError,
        model::{Exchange, Instrument, InstrumentKind, Side, SubscriptionId},
        Transformer,
    };
    use chrono::Utc;

    use super::Bitfinex;
    use crate::{
        exchange::bitfinex::{
            calculate_start_time,
            model::{BitfinexCandle, BitfinexMessage, BitfinexPayload, BitfinexTrade}, BitfinexChannel,
        },
        model::{
            subscription::{Interval, SubKind, Subscription, SubscriptionIds},
            Candle, MarketEvent, PublicTrade,
        },
        ExchangeId,
    };

    fn bitfinex(subscriptions: Vec<(Subscription, u32)>) -> Bitfinex {
        let ids = SubscriptionIds(
            subscriptions
                .into_iter()
                .map(|sub| {
                    let subscription_id = SubscriptionId::from(sub.1.to_string());
                    let sub = sub.0;
                    (subscription_id, sub)
                })
                .collect(),
        );

        Bitfinex {
            ids,
            candles_stamps: HashMap::new(),
        }
    }

    #[test]
    fn test_build_channel_meta() {
        struct TestCase {
            input: Subscription,
            expected: Result<(BitfinexChannel, String), SocketError>,
        }

        let cases = vec![
            // Candles
            TestCase {
                input: Subscription::new(
                    ExchangeId::Bitfinex,
                    ("btc", "usd", InstrumentKind::Spot),
                    SubKind::Candle(Interval::Minute1),
                ),
                expected: Ok((BitfinexChannel::Candles, "trade:1m:tBTCUSD".to_owned())),
            },
            // Trades
            TestCase {
                input: Subscription::new(
                    ExchangeId::Bitfinex,
                    ("btc", "usd", InstrumentKind::Spot),
                    SubKind::Trade,
                ),
                expected: Ok((BitfinexChannel::Trades, "tBTCUSD".to_owned())),
            },
        ];

        for (index, test) in cases.into_iter().enumerate() {
            let actual = Bitfinex::build_channel_meta(&test.input);
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
    fn test_bitfinex_transform() {
        let mut transformer = bitfinex(vec![
            (
                Subscription::from((
                    ExchangeId::Bitfinex,
                    "btc",
                    "usd",
                    InstrumentKind::Spot,
                    SubKind::Trade,
                )),
                0,
            ),
            (
                Subscription::from((
                    ExchangeId::Bitfinex,
                    "btc",
                    "usd",
                    InstrumentKind::Spot,
                    SubKind::Candle(Interval::Minute1),
                )),
                1,
            ),
        ]);

        let time = Utc::now();

        struct TestCase {
            input: BitfinexMessage,
            expected: Vec<Result<MarketEvent, SocketError>>,
        }

        let cases = vec![
            // TC0: BitfinexMessage spot trades w/ known SubscriptionId
            TestCase {
                input: BitfinexMessage {
                    channel_id: 0,
                    payload: BitfinexPayload::Trade(BitfinexTrade {
                        id: 301,
                        time,
                        side: Side::Buy,
                        price: 30.3,
                        quantity: 302.44,
                    }),
                },
                expected: vec![Ok(MarketEvent {
                    exchange_time: time,
                    received_time: time,
                    exchange: Exchange::from(ExchangeId::Bitfinex),
                    instrument: Instrument::from(("btc", "usd", InstrumentKind::Spot)),
                    kind: crate::model::DataKind::Trade(PublicTrade {
                        id: "301".to_string(),
                        price: 30.3,
                        quantity: 302.44,
                        side: Side::Buy,
                    }),
                })],
            },
            // TC1: BitfinexMessage spot candles w/ know SubscriptionId
            TestCase {
                input: BitfinexMessage {
                    channel_id: 1,
                    payload: BitfinexPayload::Candle(BitfinexCandle {
                        time,
                        open: 30.1,
                        close: 30.8,
                        high: 30.9,
                        low: 30.05,
                        volume: 3250.35,
                    }),
                },
                expected: vec![Ok(MarketEvent {
                    exchange_time: time,
                    received_time: time,
                    exchange: Exchange::from(ExchangeId::Bitfinex),
                    instrument: Instrument::from(("btc", "usd", InstrumentKind::Spot)),
                    kind: crate::model::DataKind::Candle(Candle {
                        start_time: calculate_start_time(Interval::Minute1, &time).unwrap(),
                        end_time: time,
                        open: 30.1,
                        high: 30.9,
                        low: 30.05,
                        close: 30.8,
                        volume: 3250.35,
                        trade_count: None,
                    }),
                })],
            },
        ];

        for (index, test) in cases.into_iter().enumerate() {
            let actual = transformer.transform(test.input);
            assert_eq!(
                actual.len(),
                test.expected.len(),
                "TC{index} failed at vector length assert_eq with actual: {actual:?}"
            );

            for (vector_index, (actual, expected)) in actual
                .into_iter()
                .zip(test.expected.into_iter())
                .enumerate()
            {
                match (actual, expected) {
                    (Ok(actual), Ok(expected)) => {
                        let actual = MarketEvent {
                            received_time: time,
                            ..actual
                        };
                        assert_eq!(
                            actual, expected,
                            "TC{index} failed at vector index {vector_index}"
                        );
                    }
                    // Test passed
                    (Err(_), Err(_)) => {}
                    (actual, expected) => {
                        // Test failed
                        panic!("TC{index} failed at vector index {vector_index} because actual != expected. \nActual: {actual:?}\nExpected: {expected:?}\n");
                    }
                }
            }
        }
    }
}
