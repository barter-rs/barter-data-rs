use super::super::message::GateioMessage;
use crate::{
    event::{MarketEvent, MarketIter},
    exchange::{ExchangeId, ExchangeSub},
    subscription::trade::PublicTrade,
    Identifier,
};
use barter_integration::model::{Exchange, Instrument, Side, SubscriptionId};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Terse type alias for a [`GateioFuturesUsd`](super::GateioFuturesUsd) and
/// [`GateioFuturesBtc`](super::GateioFuturesBtc) real-time trades WebSocket message.
pub type GateioFuturesTrades = GateioMessage<Vec<GateioFuturesTradeInner>>;

/// [`GateioFuturesUsd`](super::GateioFuturesUsd) and [`GateioFuturesBtc`](super::GateioFuturesBtc)
/// real-time trade WebSocket message.
///
/// ### Raw Payload Examples
/// See docs: <https://www.gate.io/docs/developers/apiv4/ws/en/#public-trades-channel>
/// ```json
/// {
///   "id": 27753479,
///   "create_time": 1545136464,
///   "create_time_ms": 1545136464123,
///   "price": "96.4",
///   "size": -108,
///   "contract": "BTC_USD"
/// }
/// ```
#[derive(Clone, PartialEq, PartialOrd, Debug, Deserialize, Serialize)]
pub struct GateioFuturesTradeInner {
    #[serde(rename = "contract")]
    pub market: String,
    #[serde(
        rename = "create_time_ms",
        deserialize_with = "barter_integration::de::de_u64_epoch_ms_as_datetime_utc"
    )]
    pub time: DateTime<Utc>,
    pub id: u64,
    #[serde(deserialize_with = "barter_integration::de::de_str")]
    pub price: f64,
    #[serde(rename = "size")]
    pub amount: f64,
}

impl Identifier<Option<SubscriptionId>> for GateioFuturesTrades {
    fn id(&self) -> Option<SubscriptionId> {
        self.data
            .first()
            .map(|trade| ExchangeSub::from((&self.channel, &trade.market)).id())
    }
}

impl From<(ExchangeId, Instrument, GateioFuturesTrades)> for MarketIter<PublicTrade> {
    fn from(
        (exchange_id, instrument, trades): (ExchangeId, Instrument, GateioFuturesTrades),
    ) -> Self {
        trades
            .data
            .into_iter()
            .map(|trade| {
                Ok(MarketEvent {
                    exchange_time: trade.time,
                    received_time: Utc::now(),
                    exchange: Exchange::from(exchange_id),
                    instrument: instrument.clone(),
                    kind: PublicTrade {
                        id: trade.id.to_string(),
                        price: trade.price,
                        amount: trade.amount,
                        side: if trade.amount.is_sign_positive() {
                            Side::Buy
                        } else {
                            Side::Sell
                        },
                    },
                })
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    mod de {
        use super::*;

        #[test]
        fn test_gateio_message_futures_trade() {
            let input = "{\"time\":1669843487,\"time_ms\":1669843487733,\"channel\":\"futures.trades\",\"event\":\"update\",\"result\":[{\"contract\":\"ETH_USDT\",\"create_time\":1669843487,\"create_time_ms\":1669843487724,\"id\":180276616,\"price\":\"1287\",\"size\":3}]}";
            serde_json::from_str::<GateioFuturesTrades>(input).unwrap();
        }
    }
}
