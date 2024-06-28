use super::OkxLevel;
use crate::{exchange::okx::book::de_subscription_id, subscription::book::OrderBookL1, Identifier};
use barter_integration::model::SubscriptionId;
use chrono::Utc;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct OkxOrderBookDataL1 {
    pub ts: String,
    pub asks: Vec<OkxLevel>,
    pub bids: Vec<OkxLevel>,
    #[serde(rename = "seqId")]
    pub seq_id: i64,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct OkxOrderBookL1 {
    #[serde(rename = "arg", deserialize_with = "de_subscription_id")]
    pub subscription_id: SubscriptionId,
    pub data: Vec<OkxOrderBookDataL1>,
}

impl Identifier<Option<SubscriptionId>> for OkxOrderBookL1 {
    fn id(&self) -> Option<SubscriptionId> {
        Some(self.subscription_id.clone())
    }
}

impl From<OkxOrderBookDataL1> for OrderBookL1 {
    fn from(data: OkxOrderBookDataL1) -> Self {
        Self {
            last_update_time: Utc::now(),
            best_bid: data.bids[0].into(),
            best_ask: data.asks[0].into(),
        }
    }
}
