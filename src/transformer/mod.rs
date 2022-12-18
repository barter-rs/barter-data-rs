use std::marker::PhantomData;
use serde::Deserialize;
use crate::{exchange::ExchangeId, Identifier, model::{Market, MarketIter}, subscriber::subscription::{SubKind, SubscriptionMap}};
use barter_integration::{
    error::SocketError,
    model::Instrument,
    Transformer,
};
use tokio::sync::mpsc;
use barter_integration::model::SubscriptionId;
use barter_integration::protocol::websocket::WsMessage;


pub trait ExchangeTransformer<Exchange, Kind>
where
    Self: Transformer<Output = Market<Kind::Event>>,
    Kind: SubKind,
{
    fn new(ws_sink_tx: mpsc::UnboundedSender<WsMessage>, map: SubscriptionMap<Exchange, Kind>) -> Self;
}

pub struct StatelessTransformer<Exchange, Kind, Input> {
    pub map: SubscriptionMap<Exchange, Kind>,
    phantom: PhantomData<Input>,
}
impl<Exchange, Kind, Input> ExchangeTransformer<Exchange, Kind> for StatelessTransformer<Exchange, Kind, Input>
where
    Kind: SubKind,
    MarketIter<Kind::Event>: From<(ExchangeId, Instrument, Input)>,
    Input: Identifier<Option<SubscriptionId>> + for<'de> Deserialize<'de>,
{
    fn new(_: mpsc::UnboundedSender<WsMessage>, map: SubscriptionMap<Exchange, Kind>) -> Self {
        Self { map, phantom: Default::default() }
    }
}

impl<Exchange, Kind, Input> Transformer for StatelessTransformer<Exchange, Kind, Input>
where
    Kind: SubKind,
    MarketIter<Kind::Event>: From<(ExchangeId, Instrument, Input)>,
    Input: Identifier<Option<SubscriptionId>> + for<'de> Deserialize<'de>,
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
            Ok(instrument) => {
                // Todo: Remove stub, remove stub when I decide how / if we bound Exchange
                MarketIter::<Kind::Event>::from((ExchangeId::Coinbase, instrument, input)).0
            }
            Err(unidentifiable) => {
                vec![Err(unidentifiable)]
            }
        }
    }
}

// // impl<Exchange, Kind, ExchangeEvent> Transformer for StatelessTransformer<Exchange, Kind, ExchangeEvent>
// impl<Exchange, Kind> Transformer for StatelessTransformer<Exchange, Kind>
// where
//     Exchange: Connector + StreamSelector<Kind>,
//     Kind: SubKind,
//     MarketIter<Kind::Event>: From<(ExchangeId, Instrument, Exchange::Message)>,
// {
//     type Input = Exchange::Message;
//     type Output = Market<Kind::Event>;
//     type OutputIter = Vec<Result<Self::Output, SocketError>>;
//
//     fn transform(&mut self, event: Self::Input) -> Self::OutputIter {
//         // Determine if the message has an identifiable SubscriptionId
//         let subscription_id = match event.id() {
//             Some(subscription_id) => subscription_id,
//             None => return vec![],
//         };
//
//         // Find Instrument associated with Input and transform
//         match self.map.find_instrument(&subscription_id) {
//             Ok(instrument) => {
//                 MarketIter::<Kind::Event>::from((Exchange::ID, instrument, event)).0
//             }
//             Err(unidentifiable) => {
//                 vec![Err(unidentifiable)]
//             }
//         }
//     }
// }
//
// // impl<Exchange, Kind, ExchangeEvent> StatelessTransformer<Exchange, Kind, ExchangeEvent> {
// impl<Exchange, Kind> StatelessTransformer<Exchange, Kind> {
//     pub fn new(map: SubscriptionMap<Exchange, Kind>) -> Self {
//         Self {
//             map,
//             // phantom: PhantomData::<ExchangeEvent>::default(),
//         }
//     }
// }
