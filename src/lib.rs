#![warn(
    // missing_debug_implementations,
    missing_copy_implementations,
    rust_2018_idioms,
    // missing_docs
)]

use std::marker::PhantomData;
use futures::{SinkExt, StreamExt};
///! # Barter-Data

use barter_integration::{ExchangeStream, Transformer};
use barter_integration::error::SocketError;
use barter_integration::protocol::websocket::{WebSocketParser, WsMessage, WsSink, WsStream};
use crate::exchange::{ExchangeId};
use crate::model::{Market, MarketIter};
use crate::subscriber::subscription::{SubKind, SubscriptionMap};
use async_trait::async_trait;
use serde::Deserialize;
use tokio::sync::mpsc;
use tracing::error;
use barter_integration::model::{Instrument, SubscriptionId};

/// Todo:
pub mod model;
pub mod subscriber;
pub mod exchange;

/// Convenient type alias for an [`ExchangeStream`] utilising a tungstenite [`WebSocket`]
// pub type ExchangeWsStream<Kind, Exchange> = ExchangeStream<WebSocketParser, WsStream, ExchangeTransformer<Kind, Exchange>>;

pub type ExchangeWsStream<Transformer> = ExchangeStream<WebSocketParser, WsStream, Transformer>;

// /// Todo: Update rust docs?
// /// [`Stream`] supertrait for streams that yield [`MarketEvent`]s. Provides an entry-point abstraction
// /// for an [`ExchangeStream`].
// #[async_trait]
// pub trait MarketStream<Kind>:
// where
//     Self: Stream<Item = Result<Market<Kind::Event>, SocketError>> + Sized + Unpin,
//     Kind: SubKind,
// {
//     /// Initialises a new [`MarketStream`] using the provided subscriptions.
//     async fn init<Exchange>(subscriptions: &[Subscription<Kind>]) -> Result<Self, SocketError>
//     where
//         Exchange: Connector,
//         Subscription<Kind>: Identifier<Exchange::Channel> + Identifier<Exchange::Market>;
// }

pub trait Identifier<T> {
    fn id(&self) -> T;
}

// pub trait ExchangeTransformer<Kind, ExchangeEvent>: Transformer<ExchangeEvent>
// where
//     Self: Transformer<ExchangeEvent, Output = Market<Kind::Event>>,
//     ExchangeEvent: for<'de> Deserialize<'de>,
//     Kind: SubKind,
// {
//     /// Constructs a new [`ExchangeTransformer`] using a transmitter to the [`WsSink`] and the
//     /// [`SubscriptionMap`].
//     ///
//     /// Note:
//     ///  - If required, the [`WsSink`] transmitter may be used to send messages to the exchange.
//     fn new(ws_sink_tx: mpsc::UnboundedSender<WsMessage>, map: SubscriptionMap<Kind>) -> Self;
// }

// pub async fn consume<Exchange, Kind>(subscriptions: Vec<Subscription<Kind>>)
// where
//     Exchange: Connector,
//     Kind: SubKind,
//     Subscription<Kind>: Identifier<Exchange::Channel> + Identifier<Exchange::Market>,
// {
//
//     let mut stream = Stream::init::<Exchange>(&subscriptions).await.unwrap();
//     while let Some(event) = stream.next().await {
//         println!("{event:?}");
//     }
// }

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

// // Todo: Try pass Exchange to init method
// #[async_trait]
// // impl<Kind, Exchange, T> MarketStream<Kind> for ExchangeWsStream<Kind, Exchange>
// impl<Kind, Transformer, ExchangeEvent> MarketStream<Kind> for ExchangeWsStream<Transformer, ExchangeEvent>
// where
//     Kind: SubKind + Send + Sync,
//     Transformer: ExchangeTransformer<Kind, ExchangeEvent>,
//     ExchangeEvent: for<'de> Deserialize<'de>,
// {
//     async fn init<Exchange>(subscriptions: &[Subscription<Kind>]) -> Result<Self, SocketError>
//     where
//         Exchange: Connector,
//         Subscription<Kind>: Identifier<Exchange::Channel> + Identifier<Exchange::Market>,
//     {
//         // Connect & subscribe
//         let (
//             websocket,
//             map
//         ) = Exchange::Subscriber::subscribe::<Kind, Exchange>(subscriptions).await?;
//
//         // Split WebSocket into WsStream & WsSink components
//         let (ws_sink, ws_stream) = websocket.split();
//
//         // Task to distribute ExchangeTransformer outgoing messages (eg/ custom pongs) to exchange
//         // --> ExchangeTransformer is operating in a synchronous trait context
//         // --> ExchangeTransformer sends messages sync via channel to async distribution task
//         // --> Async distribution tasks forwards the messages to the exchange via the ws_sink
//         let (ws_sink_tx, ws_sink_rx) = mpsc::unbounded_channel();
//         tokio::spawn(distribute_responses_to_the_exchange(
//             Exchange::ID,
//             ws_sink,
//             ws_sink_rx,
//         ));
//
//         // Construct ExchangeTransformer
//         let transformer = Transformer::new(ws_sink_tx, map);
//
//         Ok(ExchangeWsStream::new(ws_stream, transformer))
//     }
// }

/// Todo:
/// Consume [`WsMessage`]s transmitted from the [`ExchangeTransformer`] and send them on to the
/// exchange via the [`WsSink`].
///
/// If an [`ExchangeTransformer`] is required to send responses to the exchange (eg/ custom pongs),
/// it can so by transmitting the responses to the  `mpsc::UnboundedReceiver<WsMessage>` owned by
/// this asynchronous distribution task. These are then sent to the exchange via the [`WsSink`].
/// This is required because an [`ExchangeTransformer`] is operating in a synchronous trait context,
/// and therefore cannot flush the [`WsSink`] without the [`futures:task::context`].
async fn distribute_responses_to_the_exchange(
    exchange: ExchangeId,
    mut ws_sink: WsSink,
    mut ws_sink_rx: mpsc::UnboundedReceiver<WsMessage>,
) {
    while let Some(message) = ws_sink_rx.recv().await {
        if let Err(error) = ws_sink.send(message).await {
            if barter_integration::protocol::websocket::is_websocket_disconnected(&error) {
                break;
            }

            // Log error only if WsMessage failed to send over a connected WebSocket
            error!(
                %exchange,
                %error,
                "failed to send ExchangeTransformer output message to the exchange via WsSink"
            );
        }
    }
}