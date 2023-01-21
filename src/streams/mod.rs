use self::builder::StreamBuilder;
use crate::{
    exchange::ExchangeId, subscription::SubKind
};
use std::collections::HashMap;
use tokio::sync::mpsc;
use tokio_stream::{wrappers::UnboundedReceiverStream, StreamMap};

/// Defines the [`StreamBuilder`](builder::StreamBuilder) API for initialising
/// [`MarketStream`](super::MarketStream) [`Streams`].
pub mod builder;

/// Central consumer loop functionality used by the [`StreamBuilder`](builder::StreamBuilder) to
/// to drive a re-connecting [`MarketStream`](super::MarketStream).
pub mod consumer;

// Todo:
//   - Ensure rust docs are all consistent after changing from Kind::Event to T
//   - Ensure (tx, rx) naming is consistent (output_tx vs merged_tx vs joined_tx, etc)
//   - Try to reduce code duplication... while let loop is v. common

/// Ergonomic collection of exchange [`MarketEvent<T>`](MarketEvent) receivers.
#[derive(Debug)]
pub struct Streams<T> {
    pub streams: HashMap<ExchangeId, mpsc::UnboundedReceiver<T>>,
}

impl<T> Streams<T> {
    /// Construct a [`StreamBuilder`] for configuring new [`MarketEvent<T>`](MarketEvent)
    /// [`Streams`].
    pub fn builder<Kind>() -> StreamBuilder<Kind>
    where
        Kind: SubKind,
    {
        StreamBuilder::<Kind>::new()
    }

    /// Remove an exchange [`mpsc::UnboundedReceiver`] from the [`Streams`] `HashMap`.
    pub fn select(&mut self, exchange: ExchangeId) -> Option<mpsc::UnboundedReceiver<T>> {
        self.streams.remove(&exchange)
    }

    /// Join all exchange [`mpsc::UnboundedReceiver`] streams into a unified
    /// [`mpsc::UnboundedReceiver`].
    pub async fn join(self) -> mpsc::UnboundedReceiver<T>
    where
        T: Send + 'static,
    {
        let (
            output_tx,
            output_rx
        ) = mpsc::unbounded_channel();

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

    /// Join all exchange [`mpsc::UnboundedReceiver`] streams into a unified [`StreamMap`].
    pub async fn join_map(self) -> StreamMap<ExchangeId, UnboundedReceiverStream<T>> {
        self.streams
            .into_iter()
            .fold(StreamMap::new(), |mut map, (exchange, rx)| {
                map.insert(exchange, UnboundedReceiverStream::new(rx));
                map
            })
    }

    // Todo:
    //  - Naming seems off compared to join_map
    //  - ensure ExchangeChannel is used as much as possible eg/ in fn join()
    //  - should I just be using `Stream` and stream.map?
    pub async fn map<Output>(self) -> Streams<Output>
    where
        T: Send + 'static,
        Output: From<T> + Send + 'static,
    {
        let mut streams_mapped = HashMap::with_capacity(self.streams.len());

        for (exchange, mut exchange_rx) in self.streams {

            let (
                output_tx,
                output_rx
            ) = mpsc::unbounded_channel();

            streams_mapped.insert(exchange, output_rx).unwrap(); // Todo:

            tokio::spawn(async move {
                while let Some(event) = exchange_rx.recv().await {
                    let _ = output_tx.send(Output::from(event));
                }
            });
        }

        Streams {
            streams: streams_mapped
        }
    }

    // Todo:
    //  - Naming
    //  - Find way to pass an iterator of Streams...
    //  - Create re-usable function for this spawning stuff
    //  - MAKE MERGED_STREAM FROM BUILDERS W/ ONLY ONE AWAIT AFTER INIT
    //    '--> eg/ MultiStreamBuilder { futures: Vec< async builder.init() > }
    //    '--> functionality to maintain ExchangeId separation
    //    '--> Check std library iterators / streams for inspiration on best practices
    //    '--> ExchangeChannel could become more generic to be used in the multi builder
    // pub async fn merge<AnotherKind, Output>(
    //     self,
    //     other: Streams<AnotherKind>
    // ) -> MergedStreams<Output>
    // where
    //     AnotherKind: SubKind,
    //     AnotherKind::Event: Send + 'static,
    //     Kind::Event: Send + 'static,
    //     Output: FromExt<MarketEvent<Kind::Event>> + FromExt<MarketEvent<AnotherKind::Event>> + Send + 'static,
    // {
    //     let (merged_tx, merged_rx) = mpsc::unbounded_channel();
    //
    //     for mut exchange_rx in self.streams.into_values() {
    //         let merged_tx = merged_tx.clone();
    //         tokio::spawn(async move {
    //             while let Some(event) = exchange_rx.recv().await {
    //                 let _ = merged_tx.send(Output::from(event));
    //             }
    //         });
    //     }
    //
    //     for mut exchange_rx in other.streams.into_values() {
    //         let merged_tx = merged_tx.clone();
    //         tokio::spawn(async move {
    //             while let Some(event) = exchange_rx.recv().await {
    //                 let _ = merged_tx.send(Output::from(event));
    //             }
    //         });
    //     }
    //
    //     MergedStreams {
    //         merged_tx,
    //         merged_rx
    //     }
    // }
}

// /// Todo:
// #[derive(Debug)]
// pub struct MergedStreams<Output> {
//     pub merged_tx: mpsc::UnboundedSender<Output>,
//     pub merged_rx: mpsc::UnboundedReceiver<Output>
// }
//
// impl<Output> MergedStreams<Output>
// {
//     /// Todo:
//     pub async fn merge<OtherKind>(
//         self,
//         other: Streams<OtherKind>
//     ) -> Self
//     where
//         Output: FromExt<MarketEvent<OtherKind::Event>> + Send + 'static,
//         OtherKind: SubKind,
//         OtherKind::Event: Send + 'static,
//     {
//         for mut exchange_rx in other.streams.into_values() {
//             let merged_tx = self.merged_tx.clone();
//             tokio::spawn(async move {
//                 while let Some(event) = exchange_rx.recv().await {
//                     let _ = merged_tx.send(Output::from(event));
//                 }
//             });
//         }
//
//         self
//     }
// }