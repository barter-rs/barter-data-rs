use super::super::{BinanceChannel, subscription_id};
use crate::{
    Identifier,
    model::{Market, MarketIter, PublicTrade},
    subscriber::subscription::SubscriptionIdentifier,
    exchange::{
        ExchangeId,
    },
};
use barter_integration::{
    model::{Exchange, Instrument, Side, SubscriptionId},
};
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

/// Binance real-time trade message.
///
/// See docs: <https://binance-docs.github.io/apidocs/spot/en/#trade-streams>
///
/// Note:
/// For [`BinanceFuturesUsd`] this real-time stream is undocumented.
/// See discord: <https://discord.com/channels/910237311332151317/923160222711812126/975712874582388757>
#[derive(Clone, PartialEq, PartialOrd, Debug, Deserialize, Serialize)]
pub struct BinanceTrade {
    #[serde(alias = "s", deserialize_with = "de_trade_subscription_id")]
    pub subscription_id: SubscriptionId,
    #[serde(alias = "T", deserialize_with = "crate::util::de_u64_epoch_ms_as_datetime_utc")]
    pub time: DateTime<Utc>,
    #[serde(alias = "t")]
    pub id: u64,
    #[serde(alias = "p", deserialize_with = "crate::util::de_str")]
    pub price: f64,
    #[serde(alias = "q", deserialize_with = "crate::util::de_str")]
    pub amount: f64,
    #[serde(alias = "m", deserialize_with = "de_side_from_buyer_is_maker")]
    pub side: Side,
}

impl Identifier<BinanceChannel> for BinanceTrade {
    fn id() -> BinanceChannel {
        BinanceChannel::TRADES
    }
}

impl SubscriptionIdentifier for BinanceTrade {
    fn subscription_id(&self) -> SubscriptionId {
        self.subscription_id.clone()
    }
}

impl From<(ExchangeId, Instrument, BinanceTrade)> for MarketIter<PublicTrade> {
    fn from((exchange_id, instrument, trade): (ExchangeId, Instrument, BinanceTrade)) -> Self {
        Self(vec![Ok(Market {
            exchange_time: trade.time,
            received_time: Utc::now(),
            exchange: Exchange::from(exchange_id),
            instrument,
            event: PublicTrade {
                id: trade.id.to_string(),
                price: trade.price,
                amount: trade.amount,
                side: trade.side
            }
        })])
    }
}

/// Deserialize a [`BinanceTrade`] "s" (eg/ "BTCUSDT") as the associated [`SubscriptionId`]
/// (eg/ "@aggTrade|BTCUSDT").
pub fn de_trade_subscription_id<'de, D>(deserializer: D) -> Result<SubscriptionId, D::Error>
where
    D: serde::de::Deserializer<'de>,
{
    Deserialize::deserialize(deserializer)
        .map(|market| subscription_id(BinanceChannel::TRADES, market))
}

/// Deserialize a [`BinanceTrade`] "buyer_is_maker" boolean field to a Barter [`Side`].
///
/// Variants:
/// buyer_is_maker => Side::Sell
/// !buyer_is_maker => Side::Buy
pub fn de_side_from_buyer_is_maker<'de, D>(deserializer: D) -> Result<Side, D::Error>
where
    D: serde::de::Deserializer<'de>,
{
    Deserialize::deserialize(deserializer).map(|buyer_is_maker| {
        if buyer_is_maker {
            Side::Sell
        } else {
            Side::Buy
        }
    })
}