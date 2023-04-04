use barter_integration::{error::SocketError, Validator};
use serde::{Deserialize, Serialize};

/// ### Raw Payload Examples
/// See docs: <https://docs.poloniex.com/#overview-div-style-display-none-websockets-div-subscriptions-subscribe>
/// #### Subscription Trades Ok response
/// ```json
///{
///    "event": "subscribe",
///    "channel": <channel>
///}
/// ```
///
/// #### Subscription Trades Error Response
/// ```json
///{
///    "event": "error",
///    "message": "Error Message"
///}
/// ```
///

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Deserialize, Serialize)]
#[serde(tag = "event", rename_all = "lowercase")]
pub enum PoloniexSubResponse {
    #[serde(rename = "subscribe")]
    Subscribed,
    Error {
        message: String
    }
}

impl Validator for PoloniexSubResponse {
    fn validate(self) -> Result<Self, SocketError>
    where 
        Self: Sized,
    {
        match self {
            Self::Subscribed => Ok(self),
            Self::Error { message } => Err(SocketError::Subscribe(format!(
                "received failure subscription with message: {message}"
            ))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    mod de{
        use super::*;

        #[test]
        fn test_poliniex_subscription_response(){
            struct TestCase{
                input: &'static str,
                expected: Result<PoloniexSubResponse, SocketError>
            }

            let cases = vec![
                TestCase {
                    // TC0: input response is subscription success
                    input: r#"
                {
                    "event": "subscribe",
                    "channel": {"channel": "trades", "instId": "BTC-USD-191227"}
                }
                "#,
                    expected: Ok(PoloniexSubResponse::Subscribed),
                },
                TestCase {
                    // TC1: input response is failed subscription
                    input: r#"
                {
                    "event": "error",
                    "message": "Subscription failed (generic)"
                }
                "#,
                    expected: Ok(PoloniexSubResponse::Error {
                        message: "Subscription failed (generic)".to_string()
                    }),
                }
            ];

            for (index, test) in cases.into_iter().enumerate() {
                let actual = serde_json::from_str::<PoloniexSubResponse>(test.input);
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