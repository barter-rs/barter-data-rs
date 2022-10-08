use crate::{
    // exchange::{binance::futures::BinanceFuturesUsd, coinbase::Coinbase, ftx::Ftx, kraken::Kraken},
    exchange::coinbase::Coinbase,
    model::SubKind,
    ExchangeId, ExchangeWsStream, MarketEvent, MarketStream, Subscription,
};
use barter_integration::{
    error::SocketError,
    model::{InstrumentKind, Symbol},
    Event, Validator,
};
use futures::{stream::Map, StreamExt};
use std::{collections::HashMap, time::Duration};
use tokio::sync::mpsc;
use tokio_stream::{wrappers::UnboundedReceiverStream, StreamMap};
use tracing::{error, info, warn};

/// Initial duration that the [`consume`] function should wait before attempting to re-initialise
/// a [`MarketStream`]. This duration will increase exponentially as a result of repeated
/// disconnections with re-initialisation failures.
const STARTING_RECONNECT_BACKOFF_MS: u64 = 125;

/// Collection of exchange [`MarketEvent`] streams.
#[derive(Debug)]
pub struct Streams {
    pub streams: HashMap<ExchangeId, mpsc::UnboundedReceiver<Event<MarketEvent>>>,
}

impl Streams {
    /// Construct a [`StreamBuilder`] for configuring new [`MarketEvent`] [`Streams`].
    pub fn builder() -> StreamBuilder {
        StreamBuilder::new()
    }

    /// Remove an exchange [`MarketEvent`] stream from the [`Streams`] `HashMap`.
    pub fn select(
        &mut self,
        exchange: ExchangeId,
    ) -> Option<mpsc::UnboundedReceiver<Event<MarketEvent>>> {
        self.streams.remove(&exchange)
    }

    /// Join all exchange [`MarketEvent`] streams into a unified [`mpsc::UnboundedReceiver`].
    pub async fn join<Output>(self) -> mpsc::UnboundedReceiver<Output>
    where
        Output: From<Event<MarketEvent>> + Send + 'static,
    {
        let (output_tx, output_rx) = mpsc::unbounded_channel();

        for mut exchange_rx in self.streams.into_values() {
            let output_tx = output_tx.clone();
            tokio::spawn(async move {
                while let Some(event) = exchange_rx.recv().await {
                    let _ = output_tx.send(Output::from(event));
                }
            });
        }

        output_rx
    }

    /// Join all exchange [`MarketEvent`] streams into a unified [`StreamMap`].
    pub async fn join_map<Output>(
        self,
    ) -> StreamMap<
        ExchangeId,
        Map<UnboundedReceiverStream<Event<MarketEvent>>, fn(Event<MarketEvent>) -> Output>,
    >
    where
        Output: From<Event<MarketEvent>> + Send + 'static,
    {
        self.streams
            .into_iter()
            .fold(StreamMap::new(), |mut map, (exchange, rx)| {
                map.insert(exchange, UnboundedReceiverStream::new(rx).map(Output::from));
                map
            })
    }
}

/// Builder to configure and initialise [`Streams`] instances.
#[derive(Debug)]
pub struct StreamBuilder {
    pub exchange_subscriptions: HashMap<ExchangeId, Vec<Subscription>>,
}

impl StreamBuilder {
    /// Construct a new [`StreamBuilder`] instance.
    fn new() -> Self {
        Self {
            exchange_subscriptions: HashMap::new(),
        }
    }

    /// Add a collection of [`Subscription`]s to the [`StreamBuilder`]. Note that the provided
    /// [`Subscription`]s are not actioned until the [`init()`](StreamBuilder::init()) method
    /// is invoked.
    pub fn subscribe<SubIter, Sub>(mut self, subscriptions: SubIter) -> Self
    where
        SubIter: IntoIterator<Item = Sub>,
        Sub: Into<Subscription>,
    {
        subscriptions
            .into_iter()
            .map(Sub::into)
            .for_each(|subscription| {
                self.exchange_subscriptions
                    .entry(subscription.exchange)
                    .or_default()
                    .push(subscription)
            });

        self
    }

    /// Add a set of [`Subscription`]s for an exchange to the [`StreamBuilder`]. Note that the
    /// provided [`Subscription`]s are not actioned until the [`init()`](StreamBuilder::init())
    /// method is invoked.
    pub fn subscribe_exchange<SubIter, S>(
        mut self,
        exchange: ExchangeId,
        subscriptions: SubIter,
    ) -> Self
    where
        SubIter: IntoIterator<Item = (S, S, InstrumentKind, SubKind)>,
        S: Into<Symbol>,
    {
        // Construct Subscriptions from components
        let subscriptions =
            subscriptions
                .into_iter()
                .map(|(base, quote, instrument_kind, kind)| {
                    Subscription::from((exchange, base, quote, instrument_kind, kind))
                });

        // Extend collection of Subscriptions associated with the ExchangeId
        self.exchange_subscriptions
            .entry(exchange)
            .or_default()
            .extend(subscriptions);

        self
    }

    /// Spawn a [`MarketEvent`] consumer loop for each exchange. Each consumer loop distributes
    /// consumed [`MarketEvent`]s to the [`Streams`] `HashMap` (returned by this method).
    pub async fn init(mut self) -> Result<Streams, SocketError> {
        // Validate Subscriptions provided to the StreamBuilder are supported
        self = self.validate()?;

        // Construct HashMap containing a stream receiver for each ExchangeId
        let num_exchanges = self.exchange_subscriptions.len();
        let mut exchange_streams = HashMap::with_capacity(num_exchanges);

        for (exchange, mut subscriptions) in self.exchange_subscriptions {
            // Remove duplicate Subscriptions for this ExchangeId
            subscriptions.sort();
            subscriptions.dedup();

            // Create channel for this ExchangeId stream
            let (exchange_tx, exchange_rx) = mpsc::unbounded_channel();

            // Spawn a MarketStream consumer loop with this exchange's Subscriptions
            match exchange {
                // ExchangeId::BinanceFuturesUsd => {
                //     tokio::spawn(consume::<ExchangeWsStream<BinanceFuturesUsd>>(
                //         exchange,
                //         subscriptions,
                //         exchange_tx,
                //     ));
                // }
                ExchangeId::Coinbase => {
                    tokio::spawn(consume::<ExchangeWsStream<Coinbase>>(
                        exchange,
                        subscriptions,
                        exchange_tx,
                    ));
                }
                // ExchangeId::Ftx => {
                //     tokio::spawn(consume::<ExchangeWsStream<Ftx>>(
                //         exchange,
                //         subscriptions,
                //         exchange_tx,
                //     ));
                // }
                // ExchangeId::Kraken => {
                //     tokio::spawn(consume::<ExchangeWsStream<Kraken>>(
                //         exchange,
                //         subscriptions,
                //         exchange_tx,
                //     ));
                // }
                not_supported => {
                    return Err(SocketError::Subscribe(format!(
                        "Streams::init() does not support: {}",
                        not_supported
                    )))
                }
            }

            // Add exchange Event<MarketData> stream receiver to map
            exchange_streams.insert(exchange, exchange_rx);
        }

        Ok(Streams {
            streams: exchange_streams,
        })
    }
}

impl Validator for StreamBuilder {
    fn validate(self) -> Result<Self, SocketError>
    where
        Self: Sized,
    {
        // Ensure at least one exchange Subscription has been provided
        if self.exchange_subscriptions.is_empty() {
            return Err(SocketError::Subscribe(
                "StreamBuilder contains no Subscription to action".to_owned(),
            ));
        }

        // Validate each Subscription ExchangeId supports the associated Subscription kind
        self.exchange_subscriptions
            .values()
            .flatten()
            .map(|subscription| subscription.validate())
            .collect::<Result<Vec<_>, SocketError>>()?;

        Ok(self)
    }
}

/// Central [`MarketEvent`] consumer loop. Initialises an exchange [`MarketStream`] using a
/// collection of [`Subscription`]s. Consumed events are distributed downstream via the
/// `exchange_tx mpsc::UnboundedSender`. A re-connection mechanism with an exponential backoff
/// policy is utilised to ensure maximum up-time.
pub async fn consume<Stream>(
    exchange: ExchangeId,
    subscriptions: Vec<Subscription>,
    exchange_tx: mpsc::UnboundedSender<Event<MarketEvent>>,
) -> SocketError
where
    Stream: MarketStream,
{
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

        // Attempt to initialise MarketStream: if it fails on first attempt return SocketError
        let mut stream = match Stream::init(&subscriptions).await {
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

        // Consume Result<Event<MarketData>, SocketError> from MarketStream
        while let Some(event_result) = stream.next().await {
            match event_result {
                // If Ok: send Event<MarketData> to exchange receiver
                Ok(market_event) => {
                    let _ = exchange_tx.send(market_event).map_err(|err| {
                        error!(
                            payload = ?err.0,
                            why = "receiver dropped",
                            "failed to send Event<MarketData> to Exchange receiver"
                        );
                    });
                }
                // If SocketError: log & continue to next Result<Event<MarketData>, SocketError>
                Err(error) => {
                    warn!(
                        %exchange,
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
            %exchange,
            backoff_ms,
            action = "attempt re-connection after backoff",
            "exchange MarketStream unexpectedly ended"
        );
        tokio::time::sleep(Duration::from_millis(backoff_ms)).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use barter_integration::model::Instrument;

    fn stream_builder(subscription: Subscription) -> StreamBuilder {
        StreamBuilder::new().subscribe([subscription])
    }

    #[test]
    fn test_stream_builder_validate() {
        struct TestCase {
            input: StreamBuilder,
            expected: Result<StreamBuilder, SocketError>,
        }

        let cases = vec![
            TestCase {
                // TC0: Invalid StreamBuilder w/ empty subscriptions
                input: StreamBuilder::new(),
                expected: Err(SocketError::Subscribe("".to_string())),
            },
            TestCase {
                // TC1: Valid StreamBuilder w/ valid Binance Spot sub
                input: stream_builder(Subscription {
                    exchange: ExchangeId::Binance,
                    instrument: Instrument::from(("btc", "usd", InstrumentKind::Spot)),
                    kind: SubKind::Trade,
                }),
                expected: Ok(stream_builder(Subscription {
                    exchange: ExchangeId::Binance,
                    instrument: Instrument::from(("btc", "usd", InstrumentKind::Spot)),
                    kind: SubKind::Trade,
                })),
            },
            TestCase {
                // TC1: Invalid StreamBuilder w/ invalid Binance FuturePerpetual sub
                input: stream_builder(Subscription {
                    exchange: ExchangeId::Binance,
                    instrument: Instrument::from(("btc", "usd", InstrumentKind::FuturePerpetual)),
                    kind: SubKind::Trade,
                }),
                expected: Err(SocketError::Subscribe("".to_string())),
            },
        ];

        for (index, test) in cases.into_iter().enumerate() {
            let actual = test.input.validate();

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
