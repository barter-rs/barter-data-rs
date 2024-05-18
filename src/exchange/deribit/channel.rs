use crate::exchange::deribit::Deribit;
use crate::subscription::book::OrderBooksL1;
use crate::subscription::trade::PublicTrades;
use crate::subscription::Subscription;
use crate::Identifier;
use serde::Serialize;

/// Type that defines how to translate a Barter [`Subscription`] into a
/// [`Deribit`] channel to be subscribed to.
///
/// See docs: <https://docs.deribit.com/#subscriptions>
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Serialize)]
pub struct DeribitChannel<'a>(pub &'a str);

impl DeribitChannel<'_> {
    pub const TRADES: Self = Self("trades");
    pub const ORDER_BOOK_L1: Self = Self("quote");
}

impl AsRef<str> for DeribitChannel<'_> {
    fn as_ref(&self) -> &str {
        self.0
    }
}

impl<Instrument> Identifier<DeribitChannel<'static>>
    for Subscription<Deribit, Instrument, PublicTrades>
{
    fn id(&self) -> DeribitChannel<'static> {
        DeribitChannel::TRADES
    }
}

impl<Instrument> Identifier<DeribitChannel<'static>>
    for Subscription<Deribit, Instrument, OrderBooksL1>
{
    fn id(&self) -> DeribitChannel<'static> {
        DeribitChannel::ORDER_BOOK_L1
    }
}
