use super::{consumer::consume, Streams};
use crate::{
    error::DataError,
    event::MarketEvent,
    exchange::{ExchangeId, StreamSelector},
    subscription::{SubKind, Subscription},
    Identifier,
};
use barter_integration::{error::SocketError, Validator};
use std::{collections::HashMap, fmt::Debug, future::Future, pin::Pin};
use tokio::sync::mpsc;

/// Defines the [`MultiStreamBuilder`](multi::MultiStreamBuilder) API for ergonomically
/// initialising a common [`Streams<Output>`](Streams) from multiple
/// [`StreamBuilder<SubKind>`](StreamBuilder)s.
pub mod multi;

/// Communicative type alias representing the [`Future`] result of a [`Subscription`] [`validate`]
/// call generated whilst executing [`StreamBuilder::subscribe`].
pub type SubscribeFuture = Pin<Box<dyn Future<Output = Result<(), DataError>>>>;

/// Builder to configure and initialise a [`Streams<MarketEvent<SubKind::Event>`](Streams) instance
/// for a specific [`SubKind`].
#[derive(Default)]
pub struct StreamBuilder<Kind>
where
    Kind: SubKind,
{
    pub channels: HashMap<ExchangeId, ExchangeChannel<MarketEvent<Kind::Event>>>,
    pub futures: Vec<SubscribeFuture>,
}

impl<Kind> Debug for StreamBuilder<Kind>
where
    Kind: SubKind,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StreamBuilder<SubKind>")
            .field("channels", &self.channels)
            .field("num_futures", &self.futures.len())
            .finish()
    }
}

impl<Kind> StreamBuilder<Kind>
where
    Kind: SubKind,
{
    /// Construct a new [`Self`].
    pub fn new() -> Self {
        Self {
            channels: HashMap::new(),
            futures: Vec::new(),
        }
    }

    /// Add a collection of [`Subscription`]s to the [`StreamBuilder`] that will be actioned on
    /// a distinct [`WebSocket`](barter_integration::protocol::websocket::WebSocket) connection.
    ///
    /// Note that [`Subscription`]s are not actioned until the
    /// [`init()`](StreamBuilder::init()) method is invoked.
    pub fn subscribe<SubIter, Sub, Exchange>(mut self, subscriptions: SubIter) -> Self
    where
        SubIter: IntoIterator<Item = Sub>,
        Sub: Into<Subscription<Exchange, Kind>>,
        Exchange: StreamSelector<Kind> + Ord + Send + Sync + 'static,
        Kind: Ord + Send + Sync + 'static,
        Kind::Event: Send,
        Subscription<Exchange, Kind>: Identifier<Exchange::Channel> + Identifier<Exchange::Market>,
    {
        // Construct Vec<Subscriptions> from input SubIter
        let mut subscriptions = subscriptions.into_iter().map(Sub::into).collect::<Vec<_>>();

        // Acquire channel Sender to send Market<Kind::Event> from consumer loop to user
        // '--> Add ExchangeChannel Entry if this Exchange <--> SubKind combination is new
        let exchange_tx = self.channels.entry(Exchange::ID).or_default().tx.clone();

        // Add Future that once awaited will yield the Result<(), SocketError> of subscribing
        self.futures.push(Box::pin(async move {
            // Validate Subscriptions
            validate(&subscriptions)?;

            // Remove duplicate Subscriptions
            subscriptions.sort();
            subscriptions.dedup();

            // Spawn a MarketStream consumer loop with these Subscriptions<Exchange, Kind>
            tokio::spawn(consume(subscriptions, exchange_tx));

            Ok(())
        }));

        self
    }

    /// Spawn a [`MarketEvent<SubKind::Event>`](MarketEvent) consumer loop for each collection of
    /// [`Subscription`]s added to [`StreamBuilder`] via the
    /// [`subscribe()`](StreamBuilder::subscribe()) method.
    ///
    /// Each consumer loop distributes consumed [`MarketEvent<SubKind::Event>s`](MarketEvent) to
    /// the [`Streams`] `HashMap` returned by this method.
    pub async fn init(self) -> Result<Streams<MarketEvent<Kind::Event>>, DataError> {
        // Await Stream initialisation perpetual and ensure success
        futures::future::try_join_all(self.futures).await?;

        // Construct Streams using each ExchangeChannel receiver
        Ok(Streams {
            streams: self
                .channels
                .into_iter()
                .map(|(exchange, channel)| (exchange, channel.rx))
                .collect(),
        })
    }
}

/// Convenient type that holds the [`mpsc::UnboundedSender`] and [`mpsc::UnboundedReceiver`] for a
/// [`MarketEvent<T>`](MarketEvent) channel.
#[derive(Debug)]
pub struct ExchangeChannel<T> {
    tx: mpsc::UnboundedSender<T>,
    rx: mpsc::UnboundedReceiver<T>,
}

impl<T> ExchangeChannel<T> {
    /// Construct a new [`Self`].
    pub fn new() -> Self {
        let (tx, rx) = mpsc::unbounded_channel();
        Self { tx, rx }
    }
}

impl<T> Default for ExchangeChannel<T> {
    fn default() -> Self {
        Self::new()
    }
}

/// Validate the provided collection of [`Subscription`]s, ensuring that the associated exchange
/// supports every [`Subscription`] [`InstrumentKind`](barter_integration::model::InstrumentKind).
pub fn validate<Exchange, Kind>(
    subscriptions: &[Subscription<Exchange, Kind>],
) -> Result<(), DataError>
where
    Exchange: StreamSelector<Kind>,
    Kind: SubKind,
{
    // Ensure at least one Subscription has been provided
    if subscriptions.is_empty() {
        return Err(DataError::Socket(SocketError::Subscribe(
            "StreamBuilder contains no Subscription to action".to_owned(),
        )));
    }

    // Validate the Exchange supports each Subscription InstrumentKind
    subscriptions
        .iter()
        .map(|subscription| subscription.validate())
        .collect::<Result<Vec<_>, SocketError>>()?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{exchange::coinbase::Coinbase, subscription::trade::PublicTrades};
    use barter_integration::model::instrument::kind::InstrumentKind;

    #[test]
    fn test_validate() {
        struct TestCase {
            input: Vec<Subscription<Coinbase, PublicTrades>>,
            expected: Result<Vec<Subscription<Coinbase, PublicTrades>>, SocketError>,
        }

        let cases = vec![
            TestCase {
                // TC0: Invalid Vec<Subscription> w/ empty vector
                input: vec![],
                expected: Err(SocketError::Subscribe("".to_string())),
            },
            TestCase {
                // TC1: Valid Vec<Subscription> w/ valid Coinbase Spot sub
                input: vec![Subscription::from((
                    Coinbase,
                    "base",
                    "quote",
                    InstrumentKind::Spot,
                    PublicTrades,
                ))],
                expected: Ok(vec![Subscription::from((
                    Coinbase,
                    "base",
                    "quote",
                    InstrumentKind::Spot,
                    PublicTrades,
                ))]),
            },
            TestCase {
                // TC2: Invalid StreamBuilder w/ invalid Coinbase FuturePerpetual sub
                input: vec![Subscription::from((
                    Coinbase,
                    "base",
                    "quote",
                    InstrumentKind::Perpetual,
                    PublicTrades,
                ))],
                expected: Err(SocketError::Subscribe("".to_string())),
            },
        ];

        for (index, test) in cases.into_iter().enumerate() {
            let actual = validate(&test.input);

            match (actual, test.expected) {
                (Ok(_), Ok(_)) => {
                    // Test passed
                }
                (Err(_), Err(_)) => {
                    // Test passed
                }
                (actual, expected) => {
                    // Test failed
                    panic!("TC{index} failed because actual != expected. \nActual: {actual:?}\nExpected: {expected:?}\n");
                }
            }
        }
    }
}
