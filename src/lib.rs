#![warn(
    missing_debug_implementations,
    missing_copy_implementations,
    rust_2018_idioms,
    // missing_docs
)]

///! # Barter-Data

use crate::{
    model::MarketData
};
use barter_integration::{
    Subscription, SubscriptionMeta, SubscriptionIds, SubscriptionId, InstrumentKind,
    socket::{
        ExchangeSocket, Transformer,
        error::SocketError,
        protocol::websocket::{WebSocket, WebSocketParser, connect},
    }
};
use std::{
    time::Duration,
    fmt::{Display, Formatter},
};
use serde::{
    Deserialize, Serialize,
    de::DeserializeOwned
};
use async_trait::async_trait;
use futures::{SinkExt, Stream};

// Todo:
//  - Add Identifiable into ExchangeMessage bounds for ExchangeSocket & do most of transform for free
//    '--> Keep concept of ExchangeTransformer so we don't have to add From<(ExchangeId, instrument)>
//      '--> It can be a method eg/ transform<T>(&self, T) where T: Into<MarketData>
//  - Remove StreamKind & Interval from barter-integration since it's barter-data specific
//    '--> causes knock on effects... use Subscription<Kind>?


/// Core data structures to support consuming `MarketStream`s.
///
/// eg/ `MarketEvent`, `Trade`, `Subscription`, `SubscriptionId`, etc.
pub mod model;

/// Contains `Subscriber` & `ExchangeMapper` implementations for specific exchanges.
pub mod exchange;

/// Todo:
pub mod builder;

/// Custom `DataError`s generated in `barter-data`.
pub mod error;

/// Convenient type alias for an [`ExchangeSocket`] utilising a tungstenite [`WebSocket`]
pub type ExchangeWebSocket<Exchange> = ExchangeSocket<WebSocketParser, WebSocket, Exchange, MarketData>;

/// `Stream` supertrait for streams that yield [`MarketEvent`]s. Provides an entry-point abstraction
/// for an [`ExchangeWebSocket`].
#[async_trait]
pub trait MarketStream: Stream<Item = Result<MarketData, SocketError>> + Sized + Unpin {
    /// Initialises a new [`MarketEvent`] stream using the provided subscriptions.
    async fn init(subscriptions: &[Subscription]) -> Result<Self, SocketError>;
}

/// Trait that defines how a subscriber will establish a [`WebSocket`] connection with an exchange,
/// and action [`Subscription`]s. This must be implemented when integrating a new exchange.
#[async_trait]
pub trait Subscriber {
    /// Deserialisable type that this [`Subscriber`] expects to receive from the exchange in
    /// response to [`Subscription`] requests. Implements [`Validator`] in order to determine
    /// if the `SubResponse` communicates a successful outcome.
    type SubResponse: Validator + DeserializeOwned;

    /// Initialises a [`WebSocket`] connection, actions the provided collection of Barter
    /// [`Subscription`]s, and validates that the [`Subscription`] were accepted by the exchange.
    async fn subscribe(subscriptions: &[Subscription]) -> Result<(WebSocket, SubscriptionIds), SocketError> {
        // Connect to exchange
        let mut websocket = connect(Self::base_url()).await?;

        // Subscribe
        let SubscriptionMeta {
            ids,
            subscriptions,
            expected_responses,
        } = Self::build_subscription_meta(subscriptions)?;

        for subscription in subscriptions {
            websocket.send(subscription).await.map_err(SocketError::WebSocket)?;
        }

        // Validate subscriptions
        Self::validate(&mut websocket, expected_responses).await?;

        Ok((websocket, ids))
    }

    /// Returns the Base URL of the exchange to establish a connection with.
    fn base_url() -> &'static str;

    /// Uses the provided Barter [`Subscription`]s to build exchange specific subscription
    /// payloads. Generates a [`SubscriptionIds`] `Hashmap` that is used by an [`ExchangeTransformer`]
    /// to identify the Barter [`Subscription`]s associated with received messages.
    fn build_subscription_meta(
        subscriptions: &[Subscription],
    ) -> Result<SubscriptionMeta, SocketError>;


    /// Uses the provided WebSocket connection to consume [`Subscription`] responses and
    /// validate their outcomes.
    async fn validate(websocket: &mut WebSocket, expected_responses: usize) -> Result<(), SocketError> {
        todo!()
    }

    /// Return the expected `Duration` in which the exchange will respond to all actioned
    /// `WebSocket` [`Subscription`] requests.
    ///
    /// Default: 10 seconds
    fn subscription_timeout() -> Duration {
        Duration::from_secs(10)
    }
}

/// `Validator`s are capable of determining if their internal state is satisfactory to fulfill some
/// use case defined by the implementor.
pub trait Validator {
    /// Check if `Self` is valid for some use case.
    fn validate(self) -> Result<Self, SocketError>
    where
        Self: Sized;
}

/// Trait that defines how to translate between exchange specific data structures & Barter data
/// structures. This must be implemented when integrating a new exchange.
pub trait ExchangeTransformer: Transformer<MarketData> + Sized
where
    <Self as Transformer<MarketData>>::Input: Identifiable
{
    /// Unique identifier for an `ExchangeTransformer`.
    const EXCHANGE: ExchangeTransformerId;

    /// Construct a new `ExchangeTransformer` using a `HashMap` containing the relationship between
    /// all active Barter [`Subscription`]s and their associated exchange specific identifiers.
    fn new(ids: SubscriptionIds) -> Self;
}

/// Todo:
pub trait Identifiable {
    fn id(&self) -> SubscriptionId;
}

#[async_trait]
impl<Exchange> MarketStream for ExchangeWebSocket<Exchange>
where
    Exchange: Subscriber + ExchangeTransformer + Send,
    <Exchange as Transformer<MarketData>>::Input: Identifiable
{
    async fn init(subscriptions: &[Subscription]) -> Result<Self, SocketError> {
        // Connect & subscribe
        let (websocket, ids) = Exchange::subscribe(subscriptions).await?;

        // Construct ExchangeTransformer
        let transformer = Exchange::new(ids);

        Ok(ExchangeSocket::new(websocket, transformer))
    }
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
    use barter_integration::{InstrumentKind, StreamKind};

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
