use crate::event::MarketIter;
use crate::exchange::bybit::trade::BybitTrade;
use crate::exchange::ExchangeId;
use crate::subscription::trade::PublicTrade;
use crate::{exchange::bybit::channel::BybitChannel, Identifier};
use barter_integration::model::{Instrument, SubscriptionId};
use chrono::{DateTime, Utc};
use serde::{
    de::{Error, Unexpected},
    Deserialize, Serialize,
};

#[derive(Clone, PartialEq, PartialOrd, Debug, Serialize, Deserialize)]
pub struct BybitPongMessage {
    success: bool,
    ret_msg: String,
    op: String,
    conn_id: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum BybitMessage {
    Pong(BybitPongMessage),
    Trade(BybitTrade),
}

/// ### Raw Payload Examples
/// See docs: <https://bybit-exchange.github.io/docs/v5/websocket/public/trade>
/// #### Spot Side::Buy Trade
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
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Debug, Deserialize, Serialize)]
pub struct BybitPayload<T> {
    #[serde(alias = "topic", deserialize_with = "de_message_subscription_id")]
    pub subscription_id: SubscriptionId,

    #[serde(rename = "type")]
    pub r#type: String,

    #[serde(
        alias = "ts",
        deserialize_with = "barter_integration::de::de_u64_epoch_ms_as_datetime_utc"
    )]
    pub time: DateTime<Utc>,
    pub data: T,
}

/// Deserialize a [`BybitPayload`] "s" (eg/ "publicTrade.BTCUSDT") as the associated
/// [`SubscriptionId`].
///
/// eg/ "publicTrade|BTCUSDT"
pub fn de_message_subscription_id<'de, D>(deserializer: D) -> Result<SubscriptionId, D::Error>
where
    D: serde::de::Deserializer<'de>,
{
    let input = <&str as serde::Deserialize>::deserialize(deserializer)?;
    let mut tokens = input.split('.');

    match (tokens.next(), tokens.next(), tokens.next()) {
        (Some("publicTrade"), Some(market), None) => Ok(SubscriptionId::from(format!(
            "{}|{market}",
            BybitChannel::TRADES.0
        ))),
        _ => Err(Error::invalid_value(
            Unexpected::Str(input),
            &"invalid message type expected pattern: <type>.<symbol>",
        )),
    }
}

impl Identifier<Option<SubscriptionId>> for BybitMessage {
    fn id(&self) -> Option<SubscriptionId> {
        match self {
            BybitMessage::Trade(trade) => Some(trade.subscription_id.clone()),
            _ => None,
        }
    }
}

impl From<(ExchangeId, Instrument, BybitMessage)> for MarketIter<PublicTrade> {
    fn from((exchange_id, instrument, message): (ExchangeId, Instrument, BybitMessage)) -> Self {
        match message {
            BybitMessage::Pong(_m) => Self(vec![]),
            BybitMessage::Trade(trade) => Self::from((exchange_id, instrument, trade)),
        }
    }
}

impl<T> Identifier<Option<SubscriptionId>> for BybitPayload<T> {
    fn id(&self) -> Option<SubscriptionId> {
        Some(self.subscription_id.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    mod de {
        use super::*;
        use barter_integration::error::SocketError;

        #[test]
        fn test_bybit_pong() {
            struct TestCase {
                input: &'static str,
                expected: Result<BybitPongMessage, SocketError>,
            }

            let tests = vec![
                // TC0: input BybitPongMessage is deserialised
                TestCase {
                    input: r#"
                        {
                            "success": true,
                            "ret_msg": "pong",
                            "conn_id": "0970e817-426e-429a-a679-ff7f55e0b16a",
                            "op": "ping"
                        }
                    "#,
                    expected: Ok(BybitPongMessage {
                        success: true,
                        ret_msg: "pong".to_string(),
                        op: "ping".to_string(),
                        conn_id: "0970e817-426e-429a-a679-ff7f55e0b16a".to_string(),
                    }),
                },
            ];

            for (index, test) in tests.into_iter().enumerate() {
                let actual = serde_json::from_str::<BybitPongMessage>(test.input);
                match (actual, test.expected) {
                    (Ok(actual), Ok(expected)) => {
                        assert_eq!(actual, expected, "TC{} failed", index)
                    }
                    (Err(_), Err(_)) => {
                        // Test passed
                    }
                    (actual, expected) => {
                        // Test failed
                        panic!("TC{index} failed because actual != expected. \nActual: {actual:?}\nExpected: {expected:?}\n");
                    }
                }
            }
        }
    }
}
