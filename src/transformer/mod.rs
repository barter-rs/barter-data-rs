use crate::{
    exchange::ExchangeId,
    Identifier,
    model::{Market, MarketIter},
    subscriber::subscription::{SubKind, SubscriptionMap},
};
use barter_integration::{
    error::SocketError,
    model::{Instrument, SubscriptionId},
    Transformer,
};
use std::marker::PhantomData;
use serde::Deserialize;
use tokio::sync::mpsc;
use barter_integration::protocol::websocket::WsMessage;

/// Todo:
pub trait TransformerConstructor<Kind>
where
    Kind: SubKind,
{
    type Transformer: Transformer<Output = Market<Kind::Event>>;
    fn transformer(ws_sink_tx: mpsc::UnboundedSender<WsMessage>, map: SubscriptionMap<Kind>) -> Self::Transformer;
}

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
