#![warn(
    missing_debug_implementations,
    missing_copy_implementations,
    rust_2018_idioms,
    // missing_docs
)]

///! # Barter-Data

use crate::model::{MarketEvent, StreamId, StreamKind, Subscription};
use barter_integration::socket::{
    {ExchangeSocket, Transformer},
    error::SocketError,
    protocol::websocket::{connect, ExchangeWebSocket, WebSocketParser, WsMessage},
};
use std::fmt::{Display, Formatter};
use serde::{Deserialize, Serialize};
use futures::{SinkExt, Stream};
use async_trait::async_trait;
use barter_integration::{Instrument, InstrumentKind};

pub mod builder;
pub mod model;
pub mod binance;

/// `Stream` supertrait for streams that yield [`MarketEvent`]s. Provides an entry-point abstraction
/// for an [`ExchangeWebSocket`].
#[async_trait]
pub trait MarketStream: Stream<Item = Result<MarketEvent, SocketError>> + Sized + Unpin {
    /// Initialises a new [`MarketEvent`] stream using the provided subscriptions.
    async fn init(subscriptions: &[Subscription]) -> Result<Self, SocketError>;
}

/// Trait that defines how to translate between exchange specific data structures & Barter data
/// structures. This must be implemented when integrating a new exchange.
pub trait ExchangeTransformer: Sized
where
    Self: Transformer<MarketEvent>,
{
    /// Unique identifier for an `ExchangeTransformer`.
    const EXCHANGE: ExchangeTransformerId;
    /// Base URL of the exchange to establish a connection with.
    const BASE_URL: &'static str;
    fn new() -> Self;
    fn generate_subscriptions(&mut self, subscriptions: &[Subscription]) -> Vec<serde_json::Value>;
}

#[async_trait]
impl<ExchangeT> MarketStream for ExchangeWebSocket<ExchangeT, ExchangeT::Input, MarketEvent>
where
    Self: Stream<Item = Result<MarketEvent, SocketError>> + Sized + Unpin,
    ExchangeT: ExchangeTransformer + Send,
{
    async fn init(subscriptions: &[Subscription]) -> Result<Self, SocketError> {
        // Construct Exchange Transformer to translate between Barter & exchange data structures
        let mut exchange = ExchangeT::new();

        // Connect to exchange WebSocket server
        let mut websocket = connect(ExchangeT::BASE_URL).await?;

        // Action Subscriptions over the socket
        for sub_payload in exchange.generate_subscriptions(subscriptions) {
            websocket
                .send(WsMessage::Text(sub_payload.to_string()))
                .await?;
        }

        Ok(ExchangeSocket::new(websocket, WebSocketParser, exchange))
    }
}

/// `StreamIdentifier`s are capable of determine what [`StreamId`] is associated with itself.
pub trait StreamIdentifier {
    /// Return the [`StreamId`] associated with `self`.
    fn to_stream_id(&self) -> StreamId;
}

/// `Validator`s are capable of determining if their internal state is satisfactory to fulfill some
/// use case defined by the implementor.
pub trait Validator {
    /// Check if `Self` is valid for some use case.
    fn validate(self) -> Result<Self, SocketError>
    where
        Self: Sized;
}

/// Used to uniquely identify an `ExchangeTransformer` implementation. Each variant represents an
/// exchange server which can be subscribed to. Note that an exchange may have multiple servers
/// (eg/ binance, binance_futures), therefore there is a many-to-one relationship between
/// an `ExchangeId` and an exchange name.
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Deserialize, Serialize)]
pub enum ExchangeTransformerId {
    BinanceFutures,
    Binance,
    Ftx,
}

impl Display for ExchangeTransformerId {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl ExchangeTransformerId {
    /// Return the exchange name this `ExchangeTransformerId` is associated with.
    ///
    /// eg/ ExchangeTransformerId::BinanceFutures => "binance"
    pub fn exchange(&self) -> &'static str {
        match self {
            ExchangeTransformerId::Binance | ExchangeTransformerId::BinanceFutures => "binance",
            ExchangeTransformerId::Ftx => "ftx",
        }
    }

    /// Return the &str representation this `ExchangeTransformerId` is associated with.
    pub fn as_str(&self) -> &'static str {
        match self {
            ExchangeTransformerId::Binance => "binance",
            ExchangeTransformerId::BinanceFutures => "binance_futures",
            ExchangeTransformerId::Ftx => "ftx",
        }
    }

    /// Determines whether this `ExchangeTransformerId` supports the ingestion of
    /// [`InstrumentKind::Spot`](InstrumentKind) market data.
    pub fn supports_spot(&self) -> bool {
        match self {
            ExchangeTransformerId::BinanceFutures => false,
            _ => true,
        }
    }

    /// Determines whether this `ExchangeTransformerId` supports the collection of
    /// [`InstrumentKind::Future**`](InstrumentKind) market data.
    pub fn supports_futures(&self) -> bool {
        match self {
            ExchangeTransformerId::BinanceFutures => true,
            _ => false,
        }
    }
}

impl Validator for (&ExchangeTransformerId, &Vec<Subscription>) {
    fn validate(self) -> Result<Self, SocketError>
        where
            Self: Sized
    {
        let (transformer_id, subscriptions) = self;

        // Check type of InstrumentKinds associated with this ExchangeTransformer's Subscriptions
        let mut spot_subs = false;
        let mut future_subs = false;
        subscriptions
            .iter()
            .for_each(|subscription| match subscription.instrument.kind {
                InstrumentKind::Spot => spot_subs = true,
                _ => future_subs = true,
            });

        // Ensure ExchangeTransformer supports those InstrumentKinds
        let supports_spot = transformer_id.supports_spot();
        let supports_futures = transformer_id.supports_futures();
        match (supports_spot, supports_futures, spot_subs, future_subs) {
            // ExchangeTransformer has full support for all Subscription InstrumentKinds
            (true, true, _, _) => Ok(self),
            // ExchangeTransformer supports InstrumentKind::Spot, and therefore provided Subscriptions
            (true, false, true, false) => Ok(self),
            // ExchangeTransformer supports InstrumentKind::Future*, and therefore provided Subscriptions
            (false, true, false, true) => Ok(self),
            // ExchangeTransformer cannot support configured Subscriptions
            _ => Err(SocketError::Subscribe(format!(
                "ExchangeTransformer {} does not support InstrumentKinds of provided Subscriptions",
                transformer_id
            ))),
        }
    }
}

#[cfg(test)]
mod tests {
    use futures::StreamExt;
    use super::*;
    use crate::builder::Streams;
    use crate::model::{Interval, StreamKind};
    use barter_integration::InstrumentKind;

    // Todo: Maybe OutputIter will become an Option<OutputIter>?
    // Todo: Do I want to keep the name trait Exchange? Do I like the generic ExTransformer, etc.

    #[tokio::test]
    async fn stream_builder_works() -> Result<(), Box<dyn std::error::Error>> {

        let streams = Streams::builder()
            .subscribe(ExchangeTransformerId::Binance, [
                ("btc", "usdt", InstrumentKind::FuturePerpetual, StreamKind::Trades),
                ("eth", "usdt", InstrumentKind::FuturePerpetual, StreamKind::Trades),
            ])
            .subscribe(ExchangeTransformerId::Ftx, [
                ("btc", "usdt", InstrumentKind::Spot, StreamKind::Trades),
                ("eth", "usdt", InstrumentKind::Spot, StreamKind::Trades),
            ])
            .init()
            .await?;




        // Select individual exchange streams
        // let mut futures_stream = streams
        //     .select(ExchangeId::BinanceFutures)
        //     .ok_or(SocketError::Unidentifiable("".to_owned()))?; // Todo


        // let mut ftx_stream = streams
        //     .select(ExchangeId::Ftx)
        //     .ok_or(SocketError::Unidentifiable("".to_owned()))?; // Todo:

        // Join the remaining exchange streams into one
        let mut joined_stream = streams.join().await;

        while let Some((exchange, event)) = joined_stream.next().await {
            println!("{:?}", event);
        }


        Ok(())
    }
}