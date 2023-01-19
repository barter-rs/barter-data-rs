use std::any::Any;
use self::builder::StreamBuilder;
use crate::{
    event::{MarketEvent, FromExt},
    exchange::ExchangeId, subscription::SubKind
};
use std::collections::HashMap;
use tokio::sync::mpsc;
use tokio_stream::{wrappers::UnboundedReceiverStream, StreamMap};
use crate::subscription::book::OrderBooksL1;
use crate::subscription::trade::{PublicTrade, PublicTrades};

/// Defines the [`StreamBuilder`](builder::StreamBuilder) API for initialising
/// [`MarketStream`](super::MarketStream) [`Streams`].
pub mod builder;

/// Central consumer loop functionality used by the [`StreamBuilder`](builder::StreamBuilder) to
/// to drive a re-connecting [`MarketStream`](super::MarketStream).
pub mod consumer;

/// Collection of exchange [`MarketEvent<T>`](MarketEvent) streams for a specific [`SubKind`].
#[derive(Debug)]
pub struct Streams<Kind>
where
    Kind: SubKind,
{
    pub streams: HashMap<ExchangeId, mpsc::UnboundedReceiver<MarketEvent<Kind::Event>>>,
}

impl<Kind> Streams<Kind>
where
    Kind: SubKind,
{
    /// Construct a [`StreamBuilder`] for configuring new [`MarketEvent<T>`](MarketEvent) [`Streams`].
    pub fn builder() -> StreamBuilder<Kind> {
        StreamBuilder::new()
    }

    /// Remove an exchange [`MarketEvent<T>`](MarketEvent) [`mpsc::UnboundedReceiver`] from the
    /// [`Streams`] `HashMap`.
    pub fn select(
        &mut self,
        exchange: ExchangeId,
    ) -> Option<mpsc::UnboundedReceiver<MarketEvent<Kind::Event>>> {
        self.streams.remove(&exchange)
    }

    /// Join all exchange [`MarketEvent<T>`](MarketEvent) [`mpsc::UnboundedReceiver`] streams into a
    /// unified [`mpsc::UnboundedReceiver`].
    pub async fn join(self) -> mpsc::UnboundedReceiver<MarketEvent<Kind::Event>>
    where
        Kind::Event: Send + 'static,
    {
        let (output_tx, output_rx) = mpsc::unbounded_channel();

        for mut exchange_rx in self.streams.into_values() {
            let output_tx = output_tx.clone();
            tokio::spawn(async move {
                while let Some(event) = exchange_rx.recv().await {
                    let _ = output_tx.send(event);
                }
            });
        }

        output_rx
    }

    /// Join all exchange [`MarketEvent<T>`](MarketEvent) [`mpsc::UnboundedReceiver`] streams into a
    /// unified [`StreamMap`].
    pub async fn join_map(
        self,
    ) -> StreamMap<ExchangeId, UnboundedReceiverStream<MarketEvent<Kind::Event>>> {
        self.streams
            .into_iter()
            .fold(StreamMap::new(), |mut map, (exchange, rx)| {
                map.insert(exchange, UnboundedReceiverStream::new(rx));
                map
            })
    }

    /// Todo:
    ///  - Naming
    ///  - Find way to pass an iterator of Streams...
    ///  - Create re-usable function for this spawning stuff
    ///  - MAKE MERGED_STREAM FROM BUILDERS W/ ONLY ONE AWAIT AFTER INIT
    ///    '--> eg/ MultiStreamBuilder { futures: Vec< async builder.init() > }
    pub async fn merge<AnotherKind, Output>(
        self,
        other: Streams<AnotherKind>
    ) -> MergedStreams<Output>
    where
        AnotherKind: SubKind,
        AnotherKind::Event: Send + 'static,
        Kind::Event: Send + 'static,
        Output: FromExt<MarketEvent<Kind::Event>> + FromExt<MarketEvent<AnotherKind::Event>> + Send + 'static,
    {
        let (merged_tx, merged_rx) = mpsc::unbounded_channel();

        for mut exchange_rx in self.streams.into_values() {
            let merged_tx = merged_tx.clone();
            tokio::spawn(async move {
                while let Some(event) = exchange_rx.recv().await {
                    let _ = merged_tx.send(Output::from(event));
                }
            });
        }

        for mut exchange_rx in other.streams.into_values() {
            let merged_tx = merged_tx.clone();
            tokio::spawn(async move {
                while let Some(event) = exchange_rx.recv().await {
                    let _ = merged_tx.send(Output::from(event));
                }
            });
        }

        MergedStreams {
            merged_tx,
            merged_rx
        }
    }
}

/// Todo:
#[derive(Debug)]
pub struct MergedStreams<Output> {
    pub merged_tx: mpsc::UnboundedSender<Output>,
    pub merged_rx: mpsc::UnboundedReceiver<Output>
}

impl<Output> MergedStreams<Output>
{
    /// Todo:
    pub async fn merge<OtherKind>(
        self,
        other: Streams<OtherKind>
    ) -> Self
    where
        Output: FromExt<MarketEvent<OtherKind::Event>> + Send + 'static,
        OtherKind: SubKind,
        OtherKind::Event: Send + 'static,
    {
        for mut exchange_rx in other.streams.into_values() {
            let merged_tx = self.merged_tx.clone();
            tokio::spawn(async move {
                while let Some(event) = exchange_rx.recv().await {
                    let _ = merged_tx.send(Output::from(event));
                }
            });
        }

        self
    }
}