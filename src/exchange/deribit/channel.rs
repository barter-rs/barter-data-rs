use serde::Serialize;

/// Type that defines how to translate a Barter [`Subscription`] into a
/// [`Deribit`](super::Deribit) channel to be subscribed to.
///
/// See docs: <https://docs.deribit.com/#subscriptions>
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Serialize)]
pub struct DeribitChannel(pub &'static str);

impl DeribitChannel {
    pub const TRADES: Self = Self("trades");
    pub const ORDER_BOOK_L1: Self = Self("quote");
}

impl AsRef<str> for DeribitChannel {
    fn as_ref(&self) -> &str {
        self.0
    }
}
