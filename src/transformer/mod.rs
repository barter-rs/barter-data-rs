use crate::{
    exchange::{Connector, ExchangeId},
    model::{Market, MarketIter},
    subscriber::subscription::{SubKind, SubscriptionMap},
    Identifier,
};
use barter_integration::{
    error::SocketError,
    model::{Instrument, SubscriptionId},
    protocol::websocket::WsMessage,
    Transformer,
};
use serde::Deserialize;
use std::marker::PhantomData;
use tokio::sync::mpsc;

/// Generic OrderBook [`ExchangeTransformer`] implementations.
pub mod book;

/// Todo:
pub trait ExchangeTransformer<Exchange, Kind>
where
    Self: Transformer<Output = Market<Kind::Event>>,
    Kind: SubKind,
{
    /// Todo:
    fn new(
        ws_sink_tx: mpsc::UnboundedSender<WsMessage>,
        map: SubscriptionMap<Exchange, Kind>,
    ) -> Self;
}

/// Todo:
pub struct StatelessTransformer<Exchange, Kind, Input> {
    pub map: SubscriptionMap<Exchange, Kind>,
    phantom: PhantomData<Input>,
}
impl<Exchange, Kind, Input> ExchangeTransformer<Exchange, Kind> for StatelessTransformer<Exchange, Kind, Input>
where
    Exchange: Connector,
    Kind: SubKind,
    Input: Identifier<Option<SubscriptionId>> + for<'de> Deserialize<'de>,
    MarketIter<Kind::Event>: From<(ExchangeId, Instrument, Input)>,
{
    fn new(_: mpsc::UnboundedSender<WsMessage>, map: SubscriptionMap<Exchange, Kind>) -> Self {
        Self {
            map,
            phantom: Default::default(),
        }
    }
}

impl<Exchange, Kind, Input> Transformer for StatelessTransformer<Exchange, Kind, Input>
where
    Exchange: Connector,
    Kind: SubKind,
    Input: Identifier<Option<SubscriptionId>> + for<'de> Deserialize<'de>,
    MarketIter<Kind::Event>: From<(ExchangeId, Instrument, Input)>,
{
    type Input = Input;
    type Output = Market<Kind::Event>;
    type OutputIter = Vec<Result<Self::Output, SocketError>>;

    fn transform(&mut self, input: Self::Input) -> Self::OutputIter {
        // Determine if the message has an identifiable SubscriptionId
        let subscription_id = match input.id() {
            Some(subscription_id) => subscription_id,
            None => return vec![],
        };

        // Find Instrument associated with Input and transform
        match self.map.find_instrument(&subscription_id) {
            Ok(instrument) => MarketIter::<Kind::Event>::from((Exchange::ID, instrument, input)).0,
            Err(unidentifiable) => {
                vec![Err(unidentifiable)]
            }
        }
    }
}
