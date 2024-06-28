use super::{book::l1::OkxOrderBookL1, Okx};
use crate::{
    exchange::StreamSelector,
    subscription::book::{OrderBooksL1, OrderBooksL2},
    transformer::{book::MultiBookTransformer, stateless::StatelessTransformer},
    ExchangeWsStream,
};
use barter_integration::model::instrument::Instrument;

/// Level 2 OrderBook types for perpetual futures
/// [`OrderBookUpdater`](crate::transformer::book::OrderBookUpdater) implementation.
pub mod l2;

/// Level 1 OrderBook types (top of book) for perpetual futures
/// [`OrderBookUpdater`](crate::transformer::book::OrderBookUpdater) implementation.
pub mod l1;

impl StreamSelector<Instrument, OrderBooksL2> for Okx {
    type Stream = ExchangeWsStream<
        MultiBookTransformer<Self, Instrument, OrderBooksL2, l2::OkxFuturesBookUpdater>,
    >;
}

impl StreamSelector<Instrument, OrderBooksL1> for Okx {
    type Stream =
        ExchangeWsStream<StatelessTransformer<Self, Instrument, OrderBooksL1, OkxOrderBookL1>>;
}