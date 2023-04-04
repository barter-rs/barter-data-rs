use crate::{
    event::{MarketEvent, MarketIter},
    exchange::{poloniex::message::PoloniexMessage, ExchangeId},
    subscription::trade::PublicTrade,
};
use barter_integration::model::{Exchange, Instrument, Side};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Terse type alias for an [`BitmexTrade`](PoloniexTradeInner) real-time trades WebSocket message.
pub type PoloniexTrade = PoloniexMessage<PoloniexTradeInner>;

///Poloniex real-time trade websocket messsage.
///
/// ### Raw Payload Examples
/// See docs: <https://docs.poloniex.com/#public-channels-market-data-trades>
/// ```json
///{
///    "channel": "trades",
///    "data": [{
///      "symbol": "BTC_USDT",
///      "amount": "70",
///      "takerSide": "buy",
///      "quantity": "4",
///      "createTime": 1648059516810,
///      "price": "104",
///      "id": 1648059516810,
///      "ts": 1648059516832
///    }]
///}
/// ```
#[derive(Clone, PartialEq, PartialOrd, Debug, Deserialize, Serialize)]
pub struct PoloniexTradeInner {
    #[serde(alias = "symbol")]
    pub symbol: String,
    #[serde(alias = "amount", deserialize_with = "barter_integration::de::de_str")]
    pub amount: f64,
    #[serde(alias = "price", deserialize_with = "barter_integration::de::de_str")]
    pub price: f64,
    #[serde(
        alias = "ts",
        deserialize_with = "barter_integration::de::de_u64_epoch_ms_as_datetime_utc"
    )]
    pub timestamp: DateTime<Utc>,
    #[serde(alias = "id")]
    pub id: String,
    #[serde(alias = "takerSide")]
    pub side: Side,
}

impl From<(ExchangeId, Instrument, PoloniexTrade)> for MarketIter<PublicTrade> {
    fn from((exchange_id, instrument, trades): (ExchangeId, Instrument, PoloniexTrade)) -> Self {
        Self(
            trades
                .data
                .into_iter()
                .map(|trade| {
                    Ok(MarketEvent {
                        exchange_time: trade.timestamp,
                        received_time: Utc::now(),
                        exchange: Exchange::from(exchange_id),
                        instrument: instrument.clone(),
                        kind: PublicTrade {
                            id: trade.id.to_string(),
                            price: trade.price,
                            amount: trade.amount,
                            side: trade.side,
                        },
                    })
                })
                .collect(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    mod de {
        use super::*;
        use barter_integration::error::SocketError;
        use chrono::{Duration, TimeZone};

        #[test]
        fn test_poloniex_trade() {
            let input = r#"
            {
                  "symbol": "BTC_USDT",
                  "amount": "70", 
                  "takerSide": "buy",
                  "quantity": "4",
                  "createTime": 1648059516810,
                  "price": "104", 
                  "id": 1648059516810, 
                  "ts": 1648059516832
            }
            "#;

            let actual = serde_json::from_str::<PoloniexTradeInner>(input);

            let expected: Result<PoloniexTradeInner, SocketError> = Ok(PoloniexTradeInner {
                timestamp: Utc.timestamp_millis(1648059516832),
                symbol: "BTC_USDT".to_string(),
                side: Side::Buy,
                amount: 70.0,
                price: 104.0,
                id: 1648059516810,
            });

            match (actual, expected) {
                (Ok(actual), Ok(expected)) => {
                    assert_eq!(actual, expected, "TC failed")
                }
                (Err(_), Err(_)) => {
                    // Test passed
                }
                (actual, expected) => {
                    // Test failed
                    panic!("TC failed because actual != expected. \nActual: {actual:?}\nExpected: {expected:?}\n");
                }
            }
        }
    }
}
