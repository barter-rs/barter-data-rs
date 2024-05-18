use serde::{Deserialize, Serialize};

/// Type that defines how to translate a Barter [`Subscription`] into a
/// [`Deribit`] market that can be subscribed to.
///
/// See docs: <https://docs.deribit.com/#subscriptions>
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Deserialize, Serialize)]
pub struct DeribitMarket(pub String);

impl AsRef<str> for DeribitMarket {
    fn as_ref(&self) -> &str {
        self.0.as_ref()
    }
}
