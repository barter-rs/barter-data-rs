use crate::{
    exchange::ExchangeId,
    Identifier,
    model::{Market, MarketIter},
    subscriber::subscription::{SubKind, SubscriptionMap},
};
use barter_integration::{
    error::SocketError,
    model::Instrument,
    Transformer,
};
use tokio::sync::mpsc;
use barter_integration::protocol::websocket::WsMessage;
use crate::exchange::{Connector, StreamSelector};

// /// Todo:
// pub trait TransformerConstructor<Kind>
// where
//     Self: Sized,
//     Kind: SubKind,
// {
//     type Transformer: Transformer<Output = Market<Kind::Event>>;
//     fn transformer(ws_sink_tx: mpsc::UnboundedSender<WsMessage>, map: SubscriptionMap<Self, Kind>) -> Self::Transformer;
// }

pub trait ExchangeTransformer<Exchange, Kind>
where
    Self: Transformer<Output = Market<Kind::Event>>,
    Kind: SubKind,
{
    fn new(ws_sink_tx: mpsc::UnboundedSender<WsMessage>, map: SubscriptionMap<Exchange, Kind>) -> Self;
}

impl<Exchange, Kind> ExchangeTransformer<Exchange, Kind> for StatelessTransformer<Exchange, Kind>
where
    Kind: SubKind,
{
    fn new(_: mpsc::UnboundedSender<WsMessage>, map: SubscriptionMap<Exchange, Kind>) -> Self {
        Self { map }
    }
}


/// Todo:
pub struct StatelessTransformer<Exchange, Kind> {
    pub map: SubscriptionMap<Exchange, Kind>,
    // phantom: PhantomData<ExchangeEvent>,
}

// impl<Exchange, Kind, ExchangeEvent> Transformer for StatelessTransformer<Exchange, Kind, ExchangeEvent>
impl<Exchange, Kind> Transformer for StatelessTransformer<Exchange, Kind>
where
    Exchange: Connector + StreamSelector<Kind>,
    Kind: SubKind,
    MarketIter<Kind::Event>: From<(ExchangeId, Instrument, Exchange::Message)>,
{
    type Input = Exchange::Message;
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
                MarketIter::<Kind::Event>::from((Exchange::ID, instrument, event)).0
            }
            Err(unidentifiable) => {
                vec![Err(unidentifiable)]
            }
        }
    }
}

// impl<Exchange, Kind, ExchangeEvent> StatelessTransformer<Exchange, Kind, ExchangeEvent> {
impl<Exchange, Kind> StatelessTransformer<Exchange, Kind> {
    pub fn new(map: SubscriptionMap<Exchange, Kind>) -> Self {
        Self {
            map,
            // phantom: PhantomData::<ExchangeEvent>::default(),
        }
    }
}
