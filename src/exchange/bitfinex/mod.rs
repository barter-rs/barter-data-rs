//!
//! Websocket implementation for Bitfinex v2 websocket API.
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

use self::model::{
    BitfinexMessage, BitfinexPayload, BitfinexPlatformStatus, BitfinexSubResponse,
    BitfinexSubResponseKind,
};
use crate::{
    model::{
        MarketEvent,
        subscription::{Subscription, SubKind, SubscriptionIds, SubscriptionMeta},
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
use futures::StreamExt;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;
use tokio::sync::mpsc;
use tracing::debug;

/// [`Bitfinex`] specific data structures.
pub mod model;

/// `Bitfinex` [`Subscriber`] & [`ExchangeTransformer`] implementor for the collection
/// of `Spot` data.
#[derive(Clone, Eq, PartialEq, Debug, Deserialize, Serialize)]
pub struct Bitfinex {
    pub ids: SubscriptionIds,
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
                    Bitfinex::subscription_id(channel, &market),
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
                                    channel_id,
                                    key,
                                })) => {
                                    todo!()
                                    // Todo: Map candle key -> SubscriptionId
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
    /// Example Ok Return: Ok("trades", "tBTCUSD")
    /// where channel == "trades" & market == "tBTCUSD".
    fn build_channel_meta(sub: &Subscription) -> Result<(&str, String), SocketError> {
        let sub = sub.validate()?;

        // Determine Bitfinex channel
        let channel = match &sub.kind {
            SubKind::Trade => Self::CHANNEL_TRADES,
            SubKind::Candle(_) => todo!(),
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
            InstrumentKind::FuturePerpetual => todo!(),
        };

        Ok((channel, market))
    }

    /// Build a [`Bitfinex`] compatible subscription message using the channel & market provided.
    ///
    /// Example arguments: channel = "trades", market = "tBTCUSD"
    fn build_subscription_message(channel: &str, market: &str) -> WsMessage {
        WsMessage::Text(
            json!({
                "event": "subscribe",
                "channel": channel,
                "symbol": market,
            })
            .to_string(),
        )
    }
}

impl ExchangeTransformer for Bitfinex {
    const EXCHANGE: ExchangeId = ExchangeId::Bitfinex;

    fn new(_: mpsc::UnboundedSender<WsMessage>, ids: SubscriptionIds) -> Self {
        Self { ids }
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
        }
    }
}
