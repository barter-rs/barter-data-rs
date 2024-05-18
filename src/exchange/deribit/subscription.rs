use barter_integration::error::SocketError;
use barter_integration::model::SubscriptionId;
use barter_integration::Validator;
use serde::{Deserialize, Serialize};

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Deserialize, Serialize)]
pub struct DeribitResponse<T> {
    id: u32,
    result: T,
}

impl Validator for DeribitResponse<Vec<String>> {
    fn validate(self) -> Result<Self, SocketError>
    where
        Self: Sized,
    {
        if self.result.is_empty() {
            Err(SocketError::Subscribe(format!(
                "received failure subscription response: {self:?}",
            )))
        } else {
            Ok(self)
        }
    }
}
