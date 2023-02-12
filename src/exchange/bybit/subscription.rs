use barter_integration::error::SocketError;
use barter_integration::Validator;
use serde::{Deserialize, Serialize};

///```json
/// {
///     "success": true,
///     "ret_msg": "",
///     "req_id": "123",
///     "op": "subscribe",
///     "conn_id": "cejreassvfrsfvb9v1a0-2m"
/// }
///

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Deserialize, Serialize)]
pub struct BybitSubResponse {
    success: bool,
    ret_msg: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    req_id: Option<String>,
    op: String,
    conn_id: String,
}

impl Validator for BybitSubResponse {
    fn validate(self) -> Result<Self, SocketError>
        where
            Self: Sized,
    {
        if self.success {
            Ok(self)
        } else {
            Err(SocketError::Subscribe(
                "received failure subscription response".to_owned(),
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    mod de {
        use super::*;

        #[test]
        fn test_bybit_sub_response() {
            struct TestCase {
                input: &'static str,
                expected: Result<BybitSubResponse, SocketError>,
            }

            let cases = vec![
                TestCase {
                    // TC0: input response is Subscribed
                    input: r#"
                        {
                            "success": true,
                            "ret_msg": "subscribe",
                            "conn_id": "2324d924-aa4d-45b0-a858-7b8be29ab52b",
                            "req_id": "10001",
                            "op": "subscribe"
                        }
                    "#,
                    expected: Ok(BybitSubResponse {
                        success: true,
                        ret_msg: String::from("subscribe"),
                        req_id: Some(String::from("10001")),
                        op: String::from("subscribe"),
                        conn_id: String::from("2324d924-aa4d-45b0-a858-7b8be29ab52b"),
                    }),
                },
                TestCase {
                    // TC1: input response is failed subscription
                    input: r#"
                        {
                            "success": false,
                            "ret_msg": "",
                            "conn_id": "",
                            "req_id": "10001",
                            "op": ""
                        }
                    "#,
                    expected: Ok(BybitSubResponse {
                        success: false,
                        ret_msg: String::from(""),
                        req_id: Some(String::from("10001")),
                        op: String::from(""),
                        conn_id: String::from(""),
                    }),
                },
                TestCase {
                    // TC1: input response is failed subscription
                    input: r#"
                        {
                            "success": false,
                            "ret_msg": "",
                            "conn_id": "",
                            "op": ""
                        }
                    "#,
                    expected: Ok(BybitSubResponse {
                        success: false,
                        ret_msg: String::from(""),
                        req_id: None,
                        op: String::from(""),
                        conn_id: String::from(""),
                    }),
                },
            ];

            for (index, test) in cases.into_iter().enumerate() {
                let actual = serde_json::from_str::<BybitSubResponse>(test.input);
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

    #[test]
    fn test_validate_bybit_sub_response() {
        struct TestCase {
            input_response: BybitSubResponse,
            is_valid: bool,
        }

        let cases = vec![
            TestCase {
                // TC0: input response is successful subscription
                input_response: BybitSubResponse {
                    success: true,
                    ret_msg: "".to_string(),
                    req_id: None,
                    op: "".to_string(),
                    conn_id: "".to_string(),
                },
                is_valid: true,
            },
            TestCase {
                // TC1: input response is failed subscription
                input_response: BybitSubResponse {
                    success: false,
                    ret_msg: "".to_string(),
                    req_id: None,
                    op: "".to_string(),
                    conn_id: "".to_string(),
                },
                is_valid: false,
            },
        ];

        for (index, test) in cases.into_iter().enumerate() {
            let actual = test.input_response.validate().is_ok();
            assert_eq!(actual, test.is_valid, "TestCase {} failed", index);
        }
    }
}
