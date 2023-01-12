use crate::{
    event::{Market, MarketIter},
    exchange::{ExchangeId, ExchangeSub},
    subscription::trade::PublicTrade,
    Identifier,
};
use barter_integration::model::{Exchange, Instrument, Side, SubscriptionId};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Terse type alias for an [`Okx`](super::Okx) real-time trades WebSocket message.
pub type OkxTrades = OkxMessage<OkxTrade>;

/// [`Okx`](super::Okx) market data WebSocket message.
///
/// ### Raw Payload Examples
/// #### Spot Buy Trade
/// ```json
/// {
///   "arg": {
///     "channel": "trades",
///     "instId": "BTC-USDT"
///   },
///   "data": [
///     {
///       "instId": "BTC-USDT",
///       "tradeId": "130639474",
///       "px": "42219.9",
///       "sz": "0.12060306",
///       "side": "buy",
///       "ts": "1630048897897"
///     }
///   ]
/// }
/// ```
///
/// See docs: <https://www.okx.com/docs-v5/en/#websocket-api-public-channel>
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Deserialize, Serialize)]
pub struct OkxMessage<T> {
    #[serde(
        rename = "arg",
        deserialize_with = "de_okx_message_arg_as_subscription_id"
    )]
    pub subscription_id: SubscriptionId,
    pub data: Vec<T>,
}

impl<T> Identifier<Option<SubscriptionId>> for OkxMessage<T> {
    fn id(&self) -> Option<SubscriptionId> {
        Some(self.subscription_id.clone())
    }
}

/// [`Okx`](super::Okx) real-time trade WebSocket message.
///
/// ### Raw Payload Examples
/// #### Spot Buy Trade
/// ```json
/// {
///   "instId": "BTC-USDT",
///   "tradeId": "130639474",
///   "px": "42219.9",
///   "sz": "0.12060306",
///   "side": "buy",
///   "ts": "1630048897897"
/// }
/// ```
///
/// See docs: <https://www.okx.com/docs-v5/en/#websocket-api-public-channel-trades-channel>
#[derive(Clone, PartialEq, PartialOrd, Debug, Deserialize, Serialize)]
pub struct OkxTrade {
    #[serde(rename = "tradeId")]
    pub id: String,
    #[serde(rename = "px", deserialize_with = "barter_integration::de::de_str")]
    pub price: f64,
    #[serde(rename = "sz", deserialize_with = "barter_integration::de::de_str")]
    pub amount: f64,
    pub side: Side,
    #[serde(
        rename = "ts",
        deserialize_with = "barter_integration::de::de_str_u64_epoch_ms_as_datetime_utc"
    )]
    pub time: DateTime<Utc>,
}

impl From<(ExchangeId, Instrument, OkxTrades)> for MarketIter<PublicTrade> {
    fn from((exchange_id, instrument, trades): (ExchangeId, Instrument, OkxTrades)) -> Self {
        trades
            .data
            .into_iter()
            .map(|trade| {
                Ok(Market {
                    exchange_time: trade.time,
                    received_time: Utc::now(),
                    exchange: Exchange::from(exchange_id),
                    instrument: instrument.clone(),
                    event: PublicTrade {
                        id: trade.id,
                        price: trade.price,
                        amount: trade.amount,
                        side: trade.side,
                    },
                })
            })
            .collect()
    }
}

/// Deserialize an [`OkxMessage`] "arg" field as a Barter [`SubscriptionId`].
fn de_okx_message_arg_as_subscription_id<'de, D>(
    deserializer: D,
) -> Result<SubscriptionId, D::Error>
where
    D: serde::de::Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct Arg<'a> {
        channel: &'a str,
        inst_id: &'a str,
    }

    Deserialize::deserialize(deserializer)
        .map(|arg: Arg<'_>| ExchangeSub::from((arg.channel, arg.inst_id)).id())
}

#[cfg(test)]
mod tests {
    use super::*;

    // Todo:
}
