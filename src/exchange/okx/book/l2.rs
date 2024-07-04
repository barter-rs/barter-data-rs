use super::OkxLevel;
use crate::{
    exchange::okx::trade::de_okx_message_arg_as_subscription_id,
    subscription::book::{OrderBook, OrderBookSide},
    Identifier,
};
use barter_integration::model::{Side, SubscriptionId};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Copy, Clone, Deserialize, Serialize, Eq, PartialEq)]
pub enum OkxOrderBookAction {
    #[serde(rename = "snapshot")]
    SNAPSHOT,
    #[serde(rename = "update")]
    UPDATE,
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct OkxOrderBookData {
    #[serde(
        alias = "ts",
        deserialize_with = "barter_integration::de::de_str_u64_epoch_ms_as_datetime_utc"
    )]
    pub time: DateTime<Utc>,
    pub asks: Vec<OkxLevel>,
    pub bids: Vec<OkxLevel>,
    pub checksum: i64,
    #[serde(rename = "prevSeqId")]
    pub prev_seq_id: i64,
    #[serde(rename = "seqId")]
    pub seq_id: i64,
}

impl From<OkxOrderBookData> for OrderBook {
    fn from(snapshot: OkxOrderBookData) -> Self {
        Self {
            last_update_time: Utc::now(),
            bids: OrderBookSide::new(Side::Buy, snapshot.bids),
            asks: OrderBookSide::new(Side::Sell, snapshot.asks),
        }
    }
}

#[derive(Debug, Deserialize, Serialize, PartialEq)]
pub struct OkxFuturesOrderBookDelta {
    #[serde(
        rename = "arg",
        deserialize_with = "de_okx_message_arg_as_subscription_id"
    )]
    pub subscription_id: SubscriptionId,
    pub action: OkxOrderBookAction,
    pub data: Vec<OkxOrderBookData>,
}

impl Identifier<Option<SubscriptionId>> for OkxFuturesOrderBookDelta {
    fn id(&self) -> Option<SubscriptionId> {
        Some(self.subscription_id.clone())
    }
}
