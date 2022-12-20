use super::super::BinanceChannel;
use crate::{
    event::{Market, MarketIter},
    exchange::ExchangeId,
    subscription::liquidation::Liquidation,
    Identifier,
};
use barter_integration::model::{Exchange, Instrument, Side, SubscriptionId};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// [`BinanceFuturesUsd`] Liquidation order message.
///
/// See docs: <https://binance-docs.github.io/apidocs/futures/en/#liquidation-order-streams>
#[derive(Clone, PartialEq, PartialOrd, Debug, Deserialize, Serialize)]
pub struct BinanceLiquidation {
    #[serde(alias = "o")]
    pub order: BinanceLiquidationOrder,
}

/// [`BinanceFuturesUsd`] Liquidation order.
///
/// See docs: <https://binance-docs.github.io/apidocs/futures/en/#liquidation-order-streams>
#[derive(Clone, PartialEq, PartialOrd, Debug, Deserialize, Serialize)]
pub struct BinanceLiquidationOrder {
    #[serde(alias = "s", deserialize_with = "de_liquidation_subscription_id")]
    pub subscription_id: SubscriptionId,

    #[serde(alias = "S")]
    pub side: Side,

    #[serde(alias = "p", deserialize_with = "barter_integration::de::de_str")]
    pub price: f64,

    #[serde(alias = "q", deserialize_with = "barter_integration::de::de_str")]
    pub quantity: f64,

    #[serde(
        alias = "T",
        deserialize_with = "barter_integration::de::de_u64_epoch_ms_as_datetime_utc"
    )]
    pub time: DateTime<Utc>,
}

impl Identifier<Option<SubscriptionId>> for BinanceLiquidation {
    fn id(&self) -> Option<SubscriptionId> {
        Some(self.order.subscription_id.clone())
    }
}

impl From<(ExchangeId, Instrument, BinanceLiquidation)> for MarketIter<Liquidation> {
    fn from(
        (exchange_id, instrument, liquidation): (ExchangeId, Instrument, BinanceLiquidation),
    ) -> Self {
        Self(vec![Ok(Market {
            exchange_time: liquidation.order.time,
            received_time: Utc::now(),
            exchange: Exchange::from(exchange_id),
            instrument,
            event: Liquidation {
                side: liquidation.order.side,
                price: liquidation.order.price,
                quantity: liquidation.order.quantity,
                time: liquidation.order.time,
            },
        })])
    }
}

/// Deserialize a [`BinanceLiquidationOrder`] "s" (eg/ "BTCUSDT") as the associated [`SubscriptionId`]
/// (eg/ "forceOrder|BTCUSDT").
pub fn de_liquidation_subscription_id<'de, D>(deserializer: D) -> Result<SubscriptionId, D::Error>
where
    D: serde::de::Deserializer<'de>,
{
    Deserialize::deserialize(deserializer).map(|market: String| {
        SubscriptionId::from(format!("{}|{}", BinanceChannel::LIQUIDATIONS.0, market))
    })
}
