use crate::{
    DataError, ExchangeTransformerId, ExchangeWebSocket, MarketEvent, MarketStream, Subscription, Validator,
    binance::futures::BinanceFutures,
};
use std::{
    time::Duration,
    collections::HashMap
};
use futures::StreamExt;
use tokio::sync::mpsc;
use tokio_stream::{
    StreamMap, wrappers::UnboundedReceiverStream
};
use tracing::{error, info, warn};

/// Initial duration that the [`consume`] function should wait before attempting to re-initialise
/// a [`MarketStream`]. This duration will increase exponentially as a result of continued
/// disconnections.
const STARTING_RECONNECT_BACKOFF_MS: u64 = 125;

/// Collection of exchange [`MarketEvent`] streams.
#[derive(Debug)]
pub struct Streams {
    pub streams: HashMap<ExchangeTransformerId, mpsc::UnboundedReceiver<MarketEvent>>
}

impl Streams {
    /// Construct a [`StreamBuilder`] for configuring a new [`MarketEvent`] [`Streams`].
    pub fn builder() -> StreamBuilder {
        StreamBuilder::new()
    }

    /// Remove an exchange [`MarketEvent`] stream from the [`Streams`] `HashMap`.
    pub fn select(&mut self, exchange: ExchangeTransformerId) -> Option<mpsc::UnboundedReceiver<MarketEvent>> {
        self.streams
            .remove(&exchange)
    }

    /// Join all exchange [`MarketEvent`] streams into a unified stream.
    pub async fn join(self) -> StreamMap<ExchangeTransformerId, UnboundedReceiverStream<MarketEvent>> {
        self.streams
            .into_iter()
            .fold(
                StreamMap::new(),
                |mut map, (exchange, rx)| {
                    map.insert(exchange, UnboundedReceiverStream::new(rx));
                    map
                }
            )
    }
}

/// Builder to configure and initialise [`Streams`] instances.
#[derive(Debug)]
pub struct StreamBuilder {
    subscriptions: HashMap<ExchangeTransformerId, Vec<Subscription>>,
}

impl StreamBuilder {
    /// Construct a new [`StreamBuilder`] instance.
    fn new() -> Self {
        Self { subscriptions: HashMap::new() }
    }

    /// Add a set of [`Subscription`]s for an exchange to the [`StreamBuilder`]. Note
    /// that provided [`Subscription`]s are not actioned until the [`StreamBuilder::init()`](init)
    /// method is invoked.
    pub fn subscribe<SubIter, Sub>(mut self, exchange: ExchangeTransformerId, subscriptions: SubIter) -> Self
    where
        SubIter: IntoIterator<Item = Sub>,
        Sub: Into<Subscription>,
    {
        self.subscriptions
            .insert(exchange, subscriptions.into_iter().map(Sub::into).collect());

        self
    }

    /// Spawn a [`MarketEvent`] consumer loop for each exchange that distributes events to the
    /// returned [`Streams`] `HashMap`.
    pub async fn init(mut self) -> Result<Streams, DataError> {
        // Validate exchange Subscriptions provided to the StreamBuilder
        self = self.validate()?;

        // Construct Hashmap containing each Exchange's stream receiver
        let num_exchanges = self.subscriptions.len();
        let mut exchange_streams = HashMap::with_capacity(num_exchanges);

        for (exchange, subscriptions) in self.subscriptions {

            // Create channel for this exchange stream
            let (exchange_tx, exchange_rx) = mpsc::unbounded_channel();

            // Spawn a MarketStream consumer loop with this exchange's Subscriptions
            match exchange {
                ExchangeTransformerId::BinanceFutures => {
                    tokio::spawn(consume::<ExchangeWebSocket<BinanceFutures>>(exchange, subscriptions, exchange_tx));
                }
                not_supported => {
                    return Err(DataError::Subscribe(not_supported.to_string()))
                }
            }

            // Add exchange MarketEvent stream receiver to map
            exchange_streams.insert(exchange, exchange_rx);
        }

        Ok(Streams { streams: exchange_streams })
    }
}

impl Validator for StreamBuilder {
    fn validate(self) -> Result<Self, DataError>
    where
        Self: Sized
    {
        // Ensure at least one exchange Subscription has been provided
        if self.subscriptions.is_empty() {
            return Err(DataError::Subscribe(
                "StreamBuilder contains no Subscription to action".to_owned())
            )
        }

        // Ensure each ExchangeTransformer supports the provided Subscriptions
        self.subscriptions
            .iter()
            .map(|exchange_subs| exchange_subs.validate())
            .collect::<Result<Vec<_>, DataError>>()?;

        Ok(self)
    }
}

/// Central [`MarketEvent`] consumer loop. Initialises an exchange [`MarketStream`] using a
/// collection of [`Subscription`]s. Consumed events are distributed downstream via the
/// `exchange_tx mpsc::UnboundedSender`. A re-connection mechanism with an exponential backoff
/// policy is utilised to ensure maximum up-time.
pub async fn consume<Stream>(
    exchange: ExchangeTransformerId,
    subscriptions: Vec<Subscription>,
    exchange_tx: mpsc::UnboundedSender<MarketEvent>
) -> DataError
where
    Stream: MarketStream,
{
    let exchange = exchange.as_str();
    info!(
        exchange,
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
        info!(attempt, exchange, "attempting to initialise MarketStream");

        // Attempt to initialise MarketStream: if it fails on first attempt return SocketError
        let mut stream = match Stream::init(&subscriptions).await {
            Ok(stream) => {
                info!(exchange, attempt, "successfully initialised MarketStream");
                attempt = 0;
                backoff_ms = STARTING_RECONNECT_BACKOFF_MS;
                stream
            },
            Err(err) => {
                error!(
                    exchange,
                    attempt,
                    error = &*err.to_string(),
                    "failed to initialise MarketStream"
                );

                // Exit function function if Stream::init failed the first attempt, else retry
                if attempt == 1 {
                    return err
                } else {
                    continue
                }
            }
        };

        // Consume Result<MarketEvent, SocketError> from MarketStream
        while let Some(event_result) = stream.next().await {
            match event_result {
                // If Ok: send MarketEvent to exchange receiver
                Ok(market_event) => {
                    let _ = exchange_tx
                        .send(market_event)
                        .map_err(|err| {
                            error!(
                                payload = ?err.0,
                                why = "receiver dropped",
                                "failed to send MarketEvent to Exchange receiver"
                            );
                        });
                }
                // If SocketError: log & continue to next Result<MarketEvent, SocketError>
                Err(error) => {
                    warn!(
                        exchange,
                        %error,
                        action = "skipping message",
                        "consumed SocketError from MarketStream",
                    );
                    continue;
                }
            }
        }

        // If MarketStream ends unexpectedly, attempt re-connection after backoff_ms
        warn!(
            backoff_ms,
            action = "attempt re-connection after backoff",
            "exchange MarketStream unexpectedly ended"
        );
        tokio::time::sleep(Duration::from_millis(backoff_ms)).await;
    }
}