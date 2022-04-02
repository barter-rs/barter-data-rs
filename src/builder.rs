use crate::{
    ExchangeId, MarketEvent, MarketStream, Subscription,
    binance::futures::BinanceFuturesStream
};
use barter_integration::socket::error::SocketError;
use std::{
    time::Duration,
    collections::HashMap
};
use futures::StreamExt;
use tokio::sync::mpsc;
use tokio_stream::StreamMap;
use tracing::{error, info, warn};

/// Todo:
const STARTING_RECONNECT_BACKOFF_MS: u64 = 250;

/// Todo:
#[derive(Debug)]
pub struct Streams {
    pub streams: HashMap<ExchangeId, mpsc::UnboundedReceiver<MarketEvent>>
}

impl Streams {
    /// Todo:
    pub fn builder() -> StreamBuilder {
        StreamBuilder::new()
    }

    /// Todo:
    pub fn select(&mut self, exchange: ExchangeId) -> Option<mpsc::UnboundedReceiver<MarketEvent>> {
        self.streams
            .remove(&exchange)
    }

    /// Todo:
    pub async fn join(self) -> StreamMap<ExchangeId, mpsc::UnboundedReceiver<MarketEvent>> {
        self.streams
            .into_iter()
            .fold(
                StreamMap::new(),
                |mut map, (exchange, rx)| {
                    map.insert(exchange, rx);
                    map
                }
            )
    }
}

/// Todo:
#[derive(Debug)]
pub struct StreamBuilder {
    subscriptions: HashMap<ExchangeId, Vec<Subscription>>,
}

impl StreamBuilder {
    /// Todo:
    fn new() -> Self {
        Self { subscriptions: HashMap::new() }
    }

    /// Todo:
    pub fn subscribe<SubIter, Sub>(mut self, exchange: ExchangeId, subscriptions: SubIter) -> Self
        where
            SubIter: IntoIterator<Item = Sub>,
            Sub: Into<Subscription>,
    {
        self.subscriptions
            .insert(exchange, subscriptions.into_iter().map(Sub::into).collect());

        self
    }

    /// Todo:
    pub async fn init(self) -> Result<Streams, SocketError> {
        // Determine total number of exchange subscriptions
        let num_exchanges = self.subscriptions.len();
        if num_exchanges == 0 {
            return Err(SocketError::SubscribeError(
                "StreamBuilder contains no Subscription to action".to_owned())
            )
        }

        // Construct Hashmap containing each Exchange's stream receiver
        let mut exchange_streams = HashMap::with_capacity(num_exchanges);

        for (exchange, subscriptions) in self.subscriptions {

            // Create channel for this exchange stream
            let (exchange_tx, exchange_rx) = mpsc::unbounded_channel();

            // Spawn a MarketStream consumer loop with this exchange's Subscriptions
            match exchange {
                ExchangeId::BinanceFutures => {
                    tokio::spawn(consume::<BinanceFuturesStream>(exchange, subscriptions, exchange_tx));
                }
                not_supported => {
                    return Err(SocketError::SubscribeError(not_supported.to_string()))
                }
            }

            // Add exchange MarketEvent stream receiver to map
            exchange_streams.insert(exchange, exchange_rx);
        }

        Ok(Streams { streams: exchange_streams })
    }
}

/// Todo:
pub async fn consume<Stream>(
    exchange: ExchangeId,
    subscriptions: Vec<Subscription>,
    exchange_tx: mpsc::UnboundedSender<MarketEvent>
) -> SocketError
where
    Stream: MarketStream,
{
    let exchange = exchange.as_str();
    info!(
        exchange,
        subscriptions = &*format!("{:?}", subscriptions),
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
                                payload = &*format!("{:?}", err.0),
                                why = "receiver dropped",
                                "failed to send MarketEvent to Exchange receiver"
                            );
                        });
                }
                // If SocketError: log & continue to next Result<MarketEvent, SocketError>
                Err(err) => {
                    warn!(
                        exchange,
                        error = &*err.to_string(),
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