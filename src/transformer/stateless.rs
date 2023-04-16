use super::ExchangeTransformer;
use crate::{
    error::DataError,
    event::{MarketEvent, MarketIter},
    exchange::{Connector, ExchangeId},
    subscription::{Map, SubKind},
    Identifier,
};
use async_trait::async_trait;
use barter_integration::{
    model::{instrument::Instrument, SubscriptionId},
    protocol::websocket::WsMessage,
    Transformer,
};
use serde::{Deserialize, Serialize};
use std::marker::PhantomData;
use tokio::sync::mpsc;

/// Standard generic stateless [`ExchangeTransformer`] to translate exchange specific types into
/// normalised Barter types. Often used with
/// [`PublicTrades`](crate::subscription::trade::PublicTrades) or
/// [`OrderBooksL1`](crate::subscription::book::OrderBooksL1) streams.
#[derive(Clone, Eq, PartialEq, Debug, Serialize)]
pub struct StatelessTransformer<Exchange, Kind, Input> {
    instrument_map: Map<Instrument>,
    phantom: PhantomData<(Exchange, Kind, Input)>,
}

#[async_trait]
impl<Exchange, Kind, Input> ExchangeTransformer<Exchange, Kind>
    for StatelessTransformer<Exchange, Kind, Input>
where
    Exchange: Connector + Send,
    Kind: SubKind + Send,
    Input: Identifier<Option<SubscriptionId>> + for<'de> Deserialize<'de>,
    MarketIter<Kind::Event>: From<(ExchangeId, Instrument, Input)>,
{
    async fn new(
        _: mpsc::UnboundedSender<WsMessage>,
        instrument_map: Map<Instrument>,
    ) -> Result<Self, DataError> {
        Ok(Self {
            instrument_map,
            phantom: PhantomData::default(),
        })
    }
}

impl<Exchange, Kind, Input> Transformer for StatelessTransformer<Exchange, Kind, Input>
where
    Exchange: Connector,
    Kind: SubKind,
    Input: Identifier<Option<SubscriptionId>> + for<'de> Deserialize<'de>,
    MarketIter<Kind::Event>: From<(ExchangeId, Instrument, Input)>,
{
    type Error = DataError;
    type Input = Input;
    type Output = MarketEvent<Kind::Event>;
    type OutputIter = Vec<Result<Self::Output, Self::Error>>;

    fn transform(&mut self, input: Self::Input) -> Self::OutputIter {
        // Determine if the message has an identifiable SubscriptionId
        let subscription_id = match input.id() {
            Some(subscription_id) => subscription_id,
            None => return vec![],
        };

        // Find Instrument associated with Input and transform
        match self.instrument_map.find(&subscription_id) {
            Ok(instrument) => MarketIter::<Kind::Event>::from((Exchange::ID, instrument, input)).0,
            Err(unidentifiable) => vec![Err(DataError::Socket(unidentifiable))],
        }
    }
}
