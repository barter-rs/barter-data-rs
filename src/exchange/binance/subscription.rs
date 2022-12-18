use barter_integration::{error::SocketError, Validator};
use serde::{Deserialize, Serialize};

/// [`Binance`](super::Binance) subscription response message.
///
/// See docs: <https://binance-docs.github.io/apidocs/spot/en/#live-subscribing-unsubscribing-to-streams>
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Deserialize, Serialize)]
pub struct BinanceSubResponse {
    result: Option<Vec<String>>,
    id: u32,
}

impl Validator for BinanceSubResponse {
    fn validate(self) -> Result<Self, SocketError>
    where
        Self: Sized,
    {
        if self.result.is_none() {
            Ok(self)
        } else {
            Err(SocketError::Subscribe(
                "received failure subscription response".to_owned(),
            ))
        }
    }
}
