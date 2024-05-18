use crate::event::{MarketEvent, MarketIter};
use crate::exchange::deribit::message::DeribitMessage;
use crate::exchange::ExchangeId;
use crate::subscription::trade::PublicTrade;
use barter_integration::model::{Exchange, Side};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

pub type DeribitTrades = DeribitMessage<Vec<DeribitTrade>>;

/// `Deribit` real-time trade payload.
///
/// ### Raw Payload Examples
/// See docs: <https://docs.deribit.com/#trades-instrument_name-interval>
/// #### Future Buy Trade
/// ```json
/// {
///   "trade_id":"ETH-207325731"
///   "contracts":10
///   "instrument_name":"ETH-24MAY24"
///   "tick_direction":0
///   "trade_seq":14202
///   "mark_price":3132.28
///   "index_price":3126.06
///   "amount":10
///   "direction":"buy"
///   "price":3132.5
///   "timestamp":1716065232368
/// }
/// ```
///
/// #### Perpetual Sell Trade
/// ```json
///{
///  "trade_id": "ETH_USDT-1615512",
///  "contracts": 1413,
///  "instrument_name": "ETH_USDT",
///  "tick_direction": 2,
///  "trade_seq": 104577,
///  "mark_price": 3111.8099,
///  "index_price": 3111.8099,
///  "amount": 0.1413,
///  "direction": "sell",
///  "price": 3117.6,
///  "timestamp": 1716067447979
///}
/// ```
#[derive(Clone, PartialEq, PartialOrd, Debug, Deserialize, Serialize)]
pub struct DeribitTrade {
    #[serde(rename = "trade_id")]
    pub id: String,
    #[serde(rename = "timestamp", with = "chrono::serde::ts_milliseconds")]
    pub time: DateTime<Utc>,
    pub price: f64,
    pub amount: f64,
    #[serde(rename = "direction")]
    pub side: Side,
}

impl<InstrumentId: Clone> From<(ExchangeId, InstrumentId, DeribitTrades)>
    for MarketIter<InstrumentId, PublicTrade>
{
    fn from((exchange_id, instrument, trades): (ExchangeId, InstrumentId, DeribitTrades)) -> Self {
        trades
            .params
            .data
            .into_iter()
            .map(|trade| {
                Ok(MarketEvent {
                    exchange_time: trade.time,
                    received_time: Utc::now(),
                    exchange: Exchange::from(exchange_id),
                    instrument: instrument.clone(),
                    kind: PublicTrade {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::exchange::deribit::message::DeribitMessage;
    use barter_integration::model::SubscriptionId;

    #[test]
    fn test_de_deribit_message_params_trade() {
        let raw = r#"
        {
          "jsonrpc": "2.0",
          "method": "subscription",
          "params": {
            "channel": "trades.ETH-24MAY24.raw",
            "data": [
              {
                "trade_id": "ETH-207325956",
                "contracts": 10,
                "instrument_name": "ETH-24MAY24",
                "tick_direction": 2,
                "trade_seq": 14206,
                "mark_price": 3130.46,
                "index_price": 3124.5,
                "amount": 10,
                "direction": "sell",
                "price": 3131,
                "timestamp": 1716065610162
              },
              {
                "trade_id": "ETH-207325957",
                "contracts": 10,
                "instrument_name": "ETH-24MAY24",
                "tick_direction": 3,
                "trade_seq": 14207,
                "mark_price": 3130.46,
                "index_price": 3124.5,
                "amount": 10,
                "direction": "sell",
                "price": 3131,
                "timestamp": 1716065610162
              }
            ]
          }
        }
        "#;

        let value = serde_json::from_str::<DeribitMessage<Vec<DeribitTrade>>>(raw).unwrap();
        assert_eq!(
            value.params.subscription_id,
            SubscriptionId("trades|ETH-24MAY24".to_string())
        )
    }
}
