use serde::{Deserialize, Serialize};
use barter_integration::model::SubscriptionId;
use chrono::{DateTime, Utc};
use crate::exchange::bybit::channel::BybitChannel;
use crate::exchange::bybit::trade::BybitTrade;
use crate::Identifier;


/// ### Raw Payload Examples
/// See docs: <https://bybit-exchange.github.io/docs/v5/websocket/public/trade>
/// Spot Side::Buy Trade
///```json
/// {
///     "topic": "publicTrade.BTCUSDT",
///     "type": "snapshot",
///     "ts": 1672304486868,
///     "data": [
///         {
///             "T": 1672304486865,
///             "s": "BTCUSDT",
///             "S": "Buy",
///             "v": "0.001",
///             "p": "16578.50",
///             "L": "PlusTick",
///             "i": "20f43950-d8dd-5b31-9112-a178eb6023af",
///             "BT": false
///         }
///     ]
/// }
/// ```

#[derive(Clone, PartialEq, PartialOrd, Debug, Deserialize, Serialize)]
pub struct BybitMessage {
    #[serde(alias = "topic", deserialize_with = "de_message_subscription_id")]
    pub subscription_id: SubscriptionId,

    #[serde(rename = "type")]
    pub r#type: String,

    #[serde(
        alias = "ts",
        deserialize_with = "barter_integration::de::de_u64_epoch_ms_as_datetime_utc"
    )]
    pub time: DateTime<Utc>,
    pub data: Vec<BybitTrade>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub cs: Option<i32>
}

/// Deserialize a [`BinanceLiquidationOrder`] "s" (eg/ "BTCUSDT") as the associated
/// [`SubscriptionId`].
///
/// eg/ "forceOrder|BTCUSDT"
pub fn de_message_subscription_id<'de, D>(deserializer: D) -> Result<SubscriptionId, D::Error>
    where
        D: serde::de::Deserializer<'de>,
{
    Deserialize::deserialize(deserializer).map(|market: String| {
        match market {
            m if m.starts_with(BybitChannel::TRADES.0) => {
                SubscriptionId::from(format!("{}|{}", BybitChannel::TRADES.0, &m[12..]))
            },
            _ => SubscriptionId::from("unknown")
        }
    })
}

impl Identifier<Option<SubscriptionId>> for BybitMessage {
    fn id(&self) -> Option<SubscriptionId> {
        Some(self.subscription_id.clone())
    }
}
