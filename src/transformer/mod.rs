use crate::{
    exchange::ExchangeId,
    model::{Market, MarketIter},
    subscriber::subscription::{SubKind, SubscriptionMap},
    Identifier,
};
use barter_integration::{
    model::{Instrument, SubscriptionId},
    error::SocketError,
    Transformer,
};
use std::marker::PhantomData;
use serde::Deserialize;

pub struct StatelessTransformer<Kind, ExchangeEvent> {
    pub exchange_id: ExchangeId,
    pub map: SubscriptionMap<Kind>,
    phantom: PhantomData<ExchangeEvent>,
}

impl<Kind, ExchangeEvent> Transformer for StatelessTransformer<Kind, ExchangeEvent>
    where
        Kind: SubKind,
        ExchangeEvent: Identifier<Option<SubscriptionId>> + for<'de> Deserialize<'de>,
        MarketIter<Kind::Event>: From<(ExchangeId, Instrument, ExchangeEvent)>,
{
    type Input = ExchangeEvent;
    type Output = Market<Kind::Event>;
    type OutputIter = Vec<Result<Self::Output, SocketError>>;

    fn transform(&mut self, event: Self::Input) -> Self::OutputIter {
        // Determine if the message has an identifiable SubscriptionId
        let subscription_id = match event.id() {
            Some(subscription_id) => subscription_id,
            None => return vec![],
        };

        // Find Instrument associated with Input and transform
        match self.map.find_instrument(&subscription_id) {
            Ok(instrument) => {
                MarketIter::<Kind::Event>::from((self.exchange_id, instrument, event)).0
            }
            Err(unidentifiable) => {
                vec![Err(unidentifiable)]
            }
        }
    }
}

impl<Kind, ExchangeEvent> StatelessTransformer<Kind, ExchangeEvent> {
    pub fn new(exchange_id: ExchangeId, map: SubscriptionMap<Kind>) -> Self {
        Self {
            exchange_id,
            map,
            phantom: PhantomData::<ExchangeEvent>::default(),
        }
    }
}
