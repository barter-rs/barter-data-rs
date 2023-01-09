use crate::{
    error::DataError,
    event::Market,
    exchange::ExchangeId,
    subscription::{SubKind, Subscription},
    Identifier, MarketStream, StreamSelector,
};
use barter_integration::{error::SocketError, Validator};
use futures::StreamExt;
use std::fmt::Debug;
use std::{collections::HashMap, future::Future, marker::PhantomData, pin::Pin, time::Duration};
use tokio::sync::mpsc;
use tracing::{error, info, warn};

/// Initial duration that the [`consume`] function should wait after disconnecting before attempting
/// to re-initialise a [`MarketStream`]. This duration will increase exponentially as a result
/// of repeated disconnections with re-initialisation failures.
const STARTING_RECONNECT_BACKOFF_MS: u64 = 125;

/// Convenient type alias representing a [`Future`] which yields an exchange
/// [`Market<Event>`](Market) receiver.
// pub type StreamFuture<Kind> = Pin<Box<dyn Future<Output = Result<(ExchangeId, mpsc::UnboundedReceiver<Market<<Kind as SubKind>::Event>>), DataError>>>>;
pub type SubscribeFuture = Pin<Box<dyn Future<Output = Result<(), DataError>>>>;

#[derive(Debug)]
pub struct Streams<Kind>
where
    Kind: SubKind,
{
    pub streams: HashMap<ExchangeId, mpsc::UnboundedReceiver<Market<Kind::Event>>>,
}

impl<Kind> Streams<Kind>
where
    Kind: SubKind,
{
    /// Construct a [`StreamBuilder`] for configuring new [`Market<Event>`](Market) [`Streams`].
    pub fn builder() -> StreamBuilder<Kind> {
        StreamBuilder::new()
    }
}

#[derive(Default)]
pub struct StreamBuilder<Kind>
where
    Kind: SubKind,
{
    pub channels: HashMap<ExchangeId, ExchangeChannel<Kind>>,
    pub futures: Vec<SubscribeFuture>,
    phantom: PhantomData<Kind>,
}

impl<Kind> Debug for StreamBuilder<Kind>
where
    Kind: SubKind,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StreamBuilder<Kind>")
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
            phantom: PhantomData::<Kind>::default(),
        }
    }

    /// Todo:
    ///
    /// "Each call to subscribe will create a distinct WebSocket connection"
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

    /// Todo:
    pub async fn init(self) -> Result<Streams<Kind>, DataError> {
        // Await Stream initialisation futures and ensure success
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

/// Todo:
#[derive(Debug)]
pub struct ExchangeChannel<Kind>
where
    Kind: SubKind,
{
    tx: mpsc::UnboundedSender<Market<<Kind as SubKind>::Event>>,
    rx: mpsc::UnboundedReceiver<Market<<Kind as SubKind>::Event>>,
}

impl<Kind> ExchangeChannel<Kind>
where
    Kind: SubKind,
{
    /// Construct a new [`Self`].
    pub fn new() -> Self {
        let (tx, rx) = mpsc::unbounded_channel();
        Self { tx, rx }
    }
}

impl<Kind> Default for ExchangeChannel<Kind>
where
    Kind: SubKind,
{
    fn default() -> Self {
        Self::new()
    }
}

/// Todo:
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

/// Central [`MarketEvent`] consumer loop.
///
/// Initialises an exchange [`MarketStream`] using a collection of [`Subscription`]s. Consumed
/// events are distributed downstream via the `exchange_tx mpsc::UnboundedSender`. A re-connection
/// mechanism with an exponential backoff policy is utilised to ensure maximum up-time.
pub async fn consume<Exchange, Kind>(
    subscriptions: Vec<Subscription<Exchange, Kind>>,
    exchange_tx: mpsc::UnboundedSender<Market<Kind::Event>>,
) -> DataError
where
    Exchange: StreamSelector<Kind>,
    Kind: SubKind,
    Subscription<Exchange, Kind>: Identifier<Exchange::Channel> + Identifier<Exchange::Market>,
{
    // Determine ExchangeId associated with these Subscriptions
    let exchange = Exchange::ID;

    info!(
        %exchange,
        ?subscriptions,
        policy = "retry connection with exponential backoff",
        "MarketStream consumer loop running",
    );

    // Consumer loop retry parameters
    let mut attempt: u32 = 0;
    let mut backoff_ms: u64 = STARTING_RECONNECT_BACKOFF_MS;

    loop {
        // Increment retry parameters at start of every iteration
        attempt += 1;
        backoff_ms *= 2;
        info!(%exchange, attempt, "attempting to initialise MarketStream");

        // Attempt to initialise MarketStream: if it fails on first attempt return DataError
        let mut stream = match Exchange::Stream::init(&subscriptions).await {
            Ok(stream) => {
                info!(%exchange, attempt, "successfully initialised MarketStream");
                attempt = 0;
                backoff_ms = STARTING_RECONNECT_BACKOFF_MS;
                stream
            }
            Err(error) => {
                error!(%exchange, attempt, ?error, "failed to initialise MarketStream");

                // Exit function function if Stream::init failed the first attempt, else retry
                if attempt == 1 {
                    return error;
                } else {
                    continue;
                }
            }
        };

        // Consume Result<Market<Event>, DataError> from MarketStream
        while let Some(event_result) = stream.next().await {
            match event_result {
                // If Ok: send Market<Event> to exchange receiver
                Ok(market_event) => {
                    let _ = exchange_tx.send(market_event).map_err(|err| {
                        error!(
                            payload = ?err.0,
                            why = "receiver dropped",
                            "failed to send Event<MarketData> to Exchange receiver"
                        );
                    });
                }
                // If terminal DataError: break
                Err(error) if error.is_terminal() => {
                    error!(
                        %exchange,
                        %error,
                        action = "re-initialising Stream",
                        "consumed DataError from MarketStream",
                    );
                    break;
                }

                // If non-terminal DataError: log & continue
                Err(error) => {
                    warn!(
                        %exchange,
                        %error,
                        action = "skipping message",
                        "consumed DataError from MarketStream",
                    );
                    continue;
                }
            }
        }

        // If MarketStream ends unexpectedly, attempt re-connection after backoff_ms
        warn!(
            %exchange,
            backoff_ms,
            action = "attempt re-connection after backoff",
            "exchange MarketStream unexpectedly ended"
        );
        tokio::time::sleep(Duration::from_millis(backoff_ms)).await;
    }
}
