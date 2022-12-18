use super::message::GateioMessage;
use barter_integration::{error::SocketError, Validator};
use serde::{Deserialize, Serialize};

/// Todo:
pub type GateioSubResponse = GateioMessage<GateioSubResult>;

/// Todo:
///
/// See docs: <https://www.gate.io/docs/developers/apiv4/ws/en/#server-response>
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Deserialize, Serialize)]
pub struct GateioSubResult {
    pub status: String,
}

impl Validator for GateioSubResponse {
    fn validate(self) -> Result<Self, SocketError>
    where
        Self: Sized,
    {
        match &self.error {
            None => Ok(self),
            Some(failure) => Err(SocketError::Subscribe(format!(
                "received failure subscription response code: {} with message: {}",
                failure.code, failure.message,
            ))),
        }
    }
}
