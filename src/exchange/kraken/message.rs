use super::trade::KrakenTrades;
use crate::{
    event::MarketIter, exchange::ExchangeId, subscription::trade::PublicTrade, Identifier,
};
use barter_integration::model::{Instrument, SubscriptionId};
use serde::{Deserialize, Serialize};

/// [`Kraken`](super::Kraken) message variants that can be received over
/// [`WebSocket`](barter_integration::protocol::websocket::WebSocket).
///
/// ### Raw Payload Examples
/// See docs: <https://docs.kraken.com/websockets/#overview>
/// #### Trades
/// See docs: <<https://docs.kraken.com/websockets/#message-trade>
/// ```json
/// [
///     0,
///     [
///         [
///             "5541.20000",
///             "0.15850568",
///             "1534614057.321597",
///             "s",
///             "l",
///             ""
///         ],
///         [
///         "6060.00000",
///         "0.02455000",
///         "1534614057.324998",
///         "b",
///         "l",
///         ""
///         ]
///     ],
///     "trade",
///     "XBT/USD"
/// ]
/// ```
///
/// #### Heartbeat
/// See docs: <https://docs.kraken.com/websockets/#message-heartbeat>
/// ```json
/// {
///   "event": "heartbeat"
/// }
/// ```
///
/// #### KrakenError Generic
/// See docs: <https://docs.kraken.com/websockets/#errortypes>
/// ```json
/// {
///     "errorMessage": "Malformed request",
///     "event": "error"
/// }
/// ```
#[derive(Clone, PartialEq, PartialOrd, Debug, Deserialize, Serialize)]
#[serde(untagged, rename_all = "snake_case")]
pub enum KrakenMessage {
    Trades(KrakenTrades),
    Event(KrakenEvent),
}

impl Identifier<Option<SubscriptionId>> for KrakenMessage {
    fn id(&self) -> Option<SubscriptionId> {
        match self {
            KrakenMessage::Trades(trades) => Some(trades.subscription_id.clone()),
            KrakenMessage::Event(_) => None,
        }
    }
}

impl From<(ExchangeId, Instrument, KrakenMessage)> for MarketIter<PublicTrade> {
    fn from((exchange_id, instrument, message): (ExchangeId, Instrument, KrakenMessage)) -> Self {
        match message {
            KrakenMessage::Trades(trades) => Self::from((exchange_id, instrument, trades)),
            KrakenMessage::Event(_) => Self(vec![]),
        }
    }
}

/// [`Kraken`](super::Kraken) messages received over the WebSocket which are not subscription data.
///
/// eg/ [`Kraken`](super::Kraken) sends a [`KrakenEvent::Heartbeat`] if no subscription traffic
/// has been sent within the last second.
///
/// See [`KrakenMessage`] for full raw payload examples.
///
/// See docs: <https://docs.kraken.com/websockets/#message-heartbeat>
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Deserialize, Serialize)]
#[serde(tag = "event", rename_all = "camelCase")]
pub enum KrakenEvent {
    Heartbeat,
    Error(KrakenError),
}

/// [`Kraken`](super::Kraken) generic error message String received over the WebSocket.
///
/// Note that since the [`KrakenError`] is only made up of a renamed message String field, it can
/// be used flexible as a [`KrakenSubResponse::Error`](super::subscription::KrakenSubResponse)
/// or as a generic error received over the WebSocket while subscriptions are active.
///
/// See [`KrakenMessage`] for full raw payload examples.
///
/// See docs: <https://docs.kraken.com/websockets/#errortypes> <br>
/// See docs: <https://docs.kraken.com/websockets/#message-subscriptionStatus>
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Deserialize, Serialize)]
pub struct KrakenError {
    #[serde(alias = "errorMessage")]
    pub message: String,
}

#[cfg(test)]
mod tests {
    mod de {
        use crate::exchange::kraken::message::{KrakenError, KrakenEvent, KrakenMessage};
        use crate::exchange::kraken::trade::{KrakenTrade, KrakenTrades};
        use barter_integration::de::datetime_utc_from_epoch_duration;
        use barter_integration::error::SocketError;
        use barter_integration::model::{Side, SubscriptionId};

        #[test]
        fn test_kraken_message() {
            struct TestCase {
                input: &'static str,
                expected: Result<KrakenMessage, SocketError>,
            }

            let tests = vec![
                TestCase {
                    // TC0: valid KrakenMessage::Trades(KrakenTrades)
                    input: r#"
                    [
                        0,
                        [
                            [
                                "5541.20000",
                                "0.15850568",
                                "1534614057.321597",
                                "s",
                                "l",
                                ""
                            ],
                            [
                                "6060.00000",
                                "0.02455000",
                                "1534614057.324998",
                                "b",
                                "l",
                                ""
                            ]
                        ],
                      "trade",
                      "XBT/USD"
                    ]
                    "#,
                    expected: Ok(KrakenMessage::Trades(KrakenTrades {
                        subscription_id: SubscriptionId::from("trade|XBT/USD"),
                        trades: vec![
                            KrakenTrade {
                                price: 5541.2,
                                amount: 0.15850568,
                                time: datetime_utc_from_epoch_duration(
                                    std::time::Duration::from_secs_f64(1534614057.321597),
                                ),
                                side: Side::Sell,
                            },
                            KrakenTrade {
                                price: 6060.0,
                                amount: 0.02455000,
                                time: datetime_utc_from_epoch_duration(
                                    std::time::Duration::from_secs_f64(1534614057.324998),
                                ),
                                side: Side::Buy,
                            },
                        ],
                    })),
                },
                TestCase {
                    // TC1: valid KrakenMessage::Event(KrakenEvent::Heartbeat)
                    input: r#"{"event": "heartbeat"}"#,
                    expected: Ok(KrakenMessage::Event(KrakenEvent::Heartbeat)),
                },
                TestCase {
                    // TC2: valid KrakenMessage::Event(KrakenEvent::Error)
                    input: r#"
                    {
                        "errorMessage": "Malformed request",
                        "event": "error"
                    }
                    "#,
                    expected: Ok(KrakenMessage::Event(KrakenEvent::Error(KrakenError {
                        message: "Malformed request".to_string(),
                    }))),
                },
            ];

            for (index, test) in tests.into_iter().enumerate() {
                let actual = serde_json::from_str::<KrakenMessage>(test.input);
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
