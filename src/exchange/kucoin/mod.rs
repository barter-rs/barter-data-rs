//! Websocket implementation for Kucoin websocket API.
//!
//! TODO:
//! - Change exchange::get_time() to use native methods
//! - Remove all the unwraps
//! - Enforce websocket connection limits
//! - Make batch subscriptions possible.
//!
//! ## Websocket Limits
//! - 50 connections allowed per user ID. 
//! - 300 topics per websocket connection.
//! - 100 uplink messages per 10 seconds.
//! - Maximum of 100 batch subscriptions at a time.
//! 
//! ## Quirks
//! - The timestamp on trades has 6 trailing zeros, so it must be divided to yield the UNIX ms timestamp.


use std::{time::Duration, collections::HashMap};

use async_trait::async_trait;
use barter_integration::{
    error::SocketError,
    model::{InstrumentKind, SubscriptionId},
    protocol::websocket::{connect, WebSocket, WsMessage},
    Transformer, Validator,
};
use futures::SinkExt;
use rand::Rng;
use serde::{Deserialize, Serialize};
use serde_json::{from_str, json};
use tokio::sync::mpsc;

use crate::{
    exchange::get_time,
    model::{
        subscription::{SubKind, Subscription, SubscriptionIds, SubscriptionMeta, SnapshotDepth},
        MarketEvent,
    },
    ExchangeId, ExchangeTransformer, Subscriber,
};

use self::model::{KucoinMessage, KucoinSubResponse};

/// [`Kucoin`] specific data structures.
pub mod model;

/// `Kucoin` [`Subscriber`] & [`ExchangeTransformer`] implementor for the collection
/// of `Spot` data.
#[derive(Clone, Eq, PartialEq, Debug, Deserialize, Serialize)]
pub struct Kucoin {
    pub ids: SubscriptionIds,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct InstanceServers {
    endpoint: String,
    encrypt: bool,
    protocol: String,
    ping_interval: u32,
    ping_timeout: u32,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct TokenResponse {
    token: String,
    instance_servers: Vec<InstanceServers>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct KucoinTokenResponse {
    code: String,
    data: TokenResponse,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct DefaultMsg {
    pub id: String,
    pub r#type: String,
}

impl Kucoin {
    /// [`Kucoin`] trades channel name.
    ///
    /// See docs: <https://docs.kucoin.com/#match-execution-data>
    pub const CHANNEL_TRADES: &'static str = "/market/match:";
    /// [`Kucoin`] level 2 snapshot for top 5 bids and asks
    /// 
    /// see docs: <https://docs.kucoin.com/#level2-5-best-ask-bid-orders>
    pub const CHANNEL_L2SNAPSHOT_5: &'static str = "/spotMarket/level2Depth5:";
    /// [`Kucoin`] level 2 snapshot for top 50 bids and asks
    /// 
    /// see docs: <https://docs.kucoin.com/#level2-50-best-ask-bid-orders>
    pub const CHANNEL_L2SNAPSHOT_50: &'static str = "/spotMarket/level2Depth50:"; 

    /// Determine the [`Kucoin`] channel metadata associated with an input Barter [`Subscription`].
    /// This returns the topic message.
    ///
    /// Example Ok Return: Ok("/market/match:BTC-USDT")
    pub fn build_channel_metadata(sub: &Subscription) -> Result<String, SocketError> {
        let sub = sub.validate()?;

        // Determine Kucoin market using the instrument
        let market = match &sub.instrument.kind {
            InstrumentKind::Spot => format!(
                "{}-{}",
                sub.instrument.base.to_string().to_uppercase(),
                sub.instrument.quote.to_string().to_uppercase()
            ),
            InstrumentKind::FuturePerpetual => todo!(),
        };

        // Determine Kucoin channel using the Subscription SubKind
        match &sub.kind {
            SubKind::Trade => return Ok(format!("{}{}", Self::CHANNEL_TRADES, market)),
            SubKind::L2OrderBookSnapshot(depth) => match depth {
                SnapshotDepth::Depth5 => return Ok(format!("{}{}", Self::CHANNEL_L2SNAPSHOT_5, market)),
                SnapshotDepth::Depth50 => return Ok(format!("{}{}", Self::CHANNEL_L2SNAPSHOT_50, market)),
            },
            other => {
                return Err(SocketError::Unsupported {
                    entity: Self::EXCHANGE.as_str(),
                    item: other.to_string(),
                });
            }
        };
    }

    /// Build a [`Kucoin`] compatible subscription message using the topic provided.
    pub fn build_subscription_message(topic: &str) -> WsMessage {
        WsMessage::Text(
            json!({
                "id": rand::thread_rng().gen::<u16>(),
                "type": "subscribe",
                "topic": topic,
                "privateChannel": false,
                "response": true,
            })
            .to_string(),
        )
    }

    /// Build a [`Kucoin`] compatible [`SubscriptionId`] using the topic.
    /// Used to associate [`Kucoin`] data structures receive over the Websocket with
    /// the original Barter [`Subscription`].
    ///
    /// ex/ SubscriptionId("/market/match:BTC-USDT")
    /// ex/ SubscriptionId("/spotMarket/level2Depth5:BTC-USDT")
    pub fn subscription_id(topic: &str) -> SubscriptionId {
        SubscriptionId::from(topic)
    }
}

#[async_trait]
impl Subscriber for Kucoin {
    type SubResponse = KucoinSubResponse;

    // This is irrelevant
    fn base_url() -> &'static str {
        "https://api.kucoin.com"
    }

    fn build_subscription_meta(
        subscriptions: &[Subscription],
    ) -> Result<crate::model::subscription::SubscriptionMeta, SocketError> {
        let mut ids  = SubscriptionIds(HashMap::with_capacity(subscriptions.len()));

        // Map Barter subscription to Kucoin subscriptions
        let subscriptions = subscriptions.
            iter()
            .map(|subscription| {
                let topic = Self::build_channel_metadata(subscription)?;
                ids.insert(Kucoin::subscription_id(&topic), subscription.clone());

                Ok(Self::build_subscription_message(&topic))
            })
            .collect::<Result<Vec<_>, SocketError>>()?;

        Ok(SubscriptionMeta { 
            ids, 
            expected_responses: subscriptions.len(), 
            subscriptions 
        })
    }

    /// Inserts the connect url into the subscriptionIds so that when it is fed back into
    /// the ExchangeTransformer, it can start the ping loop
    async fn subscribe(
        subscriptions: &[Subscription],
    ) -> Result<(WebSocket, SubscriptionIds), SocketError> {
        let client = reqwest::Client::new();
        let resp = client
            .post("https://api.kucoin.com/api/v1/bullet-public")
            .send()
            .await
            .unwrap();
        let data: KucoinTokenResponse = from_str(resp.text().await.unwrap().as_str()).unwrap();

        // Build the connection URL
        let server = &data.data.instance_servers[0];
        let connect_id: u16 = rand::thread_rng().gen();
        let connection_url = format!(
            "{}?token={}&[connectId={}]",
            server.endpoint, data.data.token, connect_id
        );
        // Connect to exchange
        let mut websocket = connect(connection_url).await?;

        // Buld the subscription meta and subscribe to the topics
        let SubscriptionMeta {
            ids,
            subscriptions,
            expected_responses,
        } = Self::build_subscription_meta(subscriptions)?;

        for subscription in subscriptions {
            websocket.send(subscription).await?;
        }

        let ids = Self::validate(ids, &mut websocket, expected_responses).await?;

        Ok((websocket, ids))
    }
}

impl ExchangeTransformer for Kucoin {
    const EXCHANGE: ExchangeId = ExchangeId::Kucoin;

    fn new(ws_tx: mpsc::UnboundedSender<WsMessage>, ids: SubscriptionIds) -> Self {
        // Spawn a ping task
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(Duration::from_secs(30)).await;
                let ping = DefaultMsg {
                    id: get_time().to_string(),
                    r#type: "ping".to_string(),
                };

                if let Err(err) = ws_tx.send(WsMessage::Text(serde_json::to_string(&ping).unwrap())) {
                    return err;
                }
            }
        });

        Self { ids }
    }
}

impl Transformer<MarketEvent> for Kucoin {
    type Input = KucoinMessage;
    type OutputIter = Vec<Result<MarketEvent, SocketError>>;

    fn transform(&mut self, input: Self::Input) -> Self::OutputIter {
        match input {
            KucoinMessage::Trade { 
                subscription_id, 
                trade 
            } => {
                let instrument = match self.ids.find_instrument(&subscription_id) {
                    Ok(instrument) => instrument,
                    Err(error) => return vec![Err(error),]
                };

                vec![Ok(MarketEvent::from((Kucoin::EXCHANGE, instrument.clone(), trade)))]
            },
            KucoinMessage::Level2Snapshot { 
                subscription_id, 
                l2_snapshot 
            } => {
                let instrument = match self.ids.find_instrument(&subscription_id) {
                    Ok(instrument) => instrument,
                    Err(error) => return vec![Err(error),]
                };

                vec![Ok(MarketEvent::from((Kucoin::EXCHANGE, instrument.clone(), l2_snapshot)))]
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use barter_integration::model::{Side, Instrument, Exchange};
    use chrono::Utc;

    use crate::{exchange::{kucoin::model::{KucoinTrade, KucoinLevel, KucoinL2Snapshot}, datetime_utc_from_epoch_duration}, model::{PublicTrade, OrderBook, Level}};

    use super::*;

    fn kucoin(subscriptions: Vec<Subscription>) -> Kucoin {
        let ids = SubscriptionIds(
            subscriptions
                .into_iter()
                .map(|sub| {
                    let subscription_id = match (&sub.kind, &sub.instrument.kind) {
                        (SubKind::Trade, InstrumentKind::Spot) => Kucoin::subscription_id(
                            &format!("{}{}-{}", Kucoin::CHANNEL_TRADES, 
                                sub.instrument.base.to_string().to_uppercase(),
                                sub.instrument.quote.to_string().to_uppercase())
                        ),
                        (SubKind::L2OrderBookSnapshot(SnapshotDepth::Depth5), InstrumentKind::Spot) =>
                            Kucoin::subscription_id(
                                &format!("{}{}-{}",
                                Kucoin::CHANNEL_L2SNAPSHOT_5,
                                sub.instrument.base.to_string().to_uppercase(),
                                sub.instrument.quote.to_string().to_uppercase())
                            ),
                        (_, _) => {panic!("Not supported")},
                    };

                    (subscription_id, sub)
                })
                .collect(),
        );
        
        Kucoin { ids }
    }

    #[test]
    fn test_build_channel_meta() {
        struct TestCase {
            input: Subscription,
            expected: Result<String, SocketError>,
        }

        let cases = vec![
            // TC0: Supported spot trades subscription
            TestCase {
                input: Subscription::new(
                    ExchangeId::Kucoin,
                    ("btc", "usdt", InstrumentKind::Spot),
                    SubKind::Trade,
                ),
                expected: Ok("/market/match:BTC-USDT".to_string()),
            }
        ];

        for (index, test) in cases.into_iter().enumerate() {
            let actual = Kucoin::build_channel_metadata(&test.input);
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
    fn test_kucoin_transform() {
        let mut transformer = kucoin(vec![
            Subscription::from((
                ExchangeId::Kucoin,
                "btc",
                "usdt",
                InstrumentKind::Spot,
                SubKind::Trade,
            )),
            Subscription::from((
                ExchangeId::Kucoin,
                "btc",
                "usdt",
                InstrumentKind::Spot,
                SubKind::L2OrderBookSnapshot(SnapshotDepth::Depth5)
            ))
        ]);

        let time = Utc::now();

        struct TestCase {
            input: KucoinMessage,
            expected: Vec<Result<MarketEvent, SocketError>>,
        }

        let cases = vec![
            TestCase {
                // TC0: KucoinMessage with unknown SubscriptionId
                input: KucoinMessage::Trade { 
                    subscription_id: SubscriptionId::from("unknown"), 
                    trade: KucoinTrade { 
                        sequence: 1, 
                        maker_order_id: String::from("me"), 
                        taker_order_id: String::from("you"), 
                        symbol: String::from("BTC-USDT"), 
                        trade_type: String::from("match"), 
                        trade_id: 1, 
                        price: 20.0, 
                        side: Side::Buy, 
                        size: 0.9, 
                        time 
                    }
                },
                expected: vec![Err(SocketError::Unidentifiable(SubscriptionId::from("unknown")))],
            },
            TestCase {
                // TC1: KucoinMessage spot trade w/ known SubscriptionId
                input: KucoinMessage::Trade {
                    subscription_id: SubscriptionId::from("/market/match:BTC-USDT"),
                    trade: KucoinTrade { 
                        sequence: 1, 
                        maker_order_id: String::from("me"), 
                        taker_order_id: String::from("you"), 
                        symbol: String::from("BTC-USDT"), 
                        trade_type: String::from("match"), 
                        trade_id: 1, 
                        price: 20.0, 
                        side: Side::Buy, 
                        size: 0.9, 
                        time 
                    }
                },
                expected: vec![Ok(MarketEvent { 
                    exchange_time: time, 
                    received_time: time, 
                    exchange: Exchange::from(ExchangeId::Kucoin), 
                    instrument: Instrument::from(("btc", "usdt", InstrumentKind::Spot)), 
                    kind: crate::model::DataKind::Trade(PublicTrade {
                        id: "1".to_string(),
                        price: 20.0,
                        quantity: 0.9,
                        side: Side::Buy,
                    }) 
                })]
            },
            TestCase {
                // TC2: KucoinMessage L2 orderbook snapshot w/ known SubscriptionId
                input: KucoinMessage::Level2Snapshot { 
                    subscription_id: SubscriptionId::from("/spotMarket/level2Depth5:BTC-USDT"), 
                    l2_snapshot: KucoinL2Snapshot { 
                        time: datetime_utc_from_epoch_duration(Duration::from_millis(1665674855819)), 
                        asks: vec![
                            KucoinLevel { price: 18800.9, quantity: 0.9175},
                            KucoinLevel { price: 18801.0, quantity: 0.0675}
                        ], 
                        bids: vec![
                            KucoinLevel { price: 18795.1, quantity: 0.0828},
                            KucoinLevel { price: 18794.6, quantity: 0.05422529},
                        ] 
                    }
                },
                expected: vec![Ok(MarketEvent { 
                    exchange_time: datetime_utc_from_epoch_duration(Duration::from_millis(1665674855819)), 
                    received_time: time, 
                    exchange: Exchange::from(ExchangeId::Kucoin), 
                    instrument: Instrument::from(("btc", "usdt", InstrumentKind::Spot)), 
                    kind: crate::model::DataKind::OrderBook(OrderBook { 
                        last_update_id: 0, 
                        asks: vec![
                            Level { price: 18800.9, quantity: 0.9175 },
                            Level { price: 18801.0, quantity: 0.0675 }
                        ], 
                        bids: vec![
                            Level { price: 18795.1, quantity: 0.0828 },
                            Level { price: 18794.6, quantity: 0.05422529}
                        ] }) 
                })]
            }
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
