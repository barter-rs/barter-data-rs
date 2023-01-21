use crate::error::DataError;
use std::{future::Future, pin::Pin};
use std::collections::HashMap;
use crate::event::{FromExt, MarketEvent};
use crate::exchange::ExchangeId;
use crate::streams::builder::{ExchangeChannel, StreamBuilder};
use crate::streams::Streams;
use crate::subscription::SubKind;

/// Todo:
pub type BuilderInitFuture = Pin<Box<dyn Future<Output = Result<(), DataError>>>>;

/// Todo:
///  - Should probably just be StreamBuilder... that olds some Inner? Or Streams::builder returns Multi
#[derive(Default)]
pub struct MultiStreamBuilder<Output> {
    pub channels: HashMap<ExchangeId, ExchangeChannel<Output>>,
    pub futures: Vec<BuilderInitFuture>,
}

impl<Output> MultiStreamBuilder<Output> {
    pub fn new() -> Self {
        Self {
            channels: HashMap::new(),
            futures: Vec::new(),
        }
    }

    pub fn add<Kind>(mut self, builder: StreamBuilder<Kind>) -> Self
    where
        Output: FromExt<MarketEvent<Kind::Event>> + Send + 'static,
        Kind: SubKind + 'static,
        Kind::Event: Send
    {
        let mut exchange_txs = HashMap::with_capacity(builder.channels.len());

        for exchange in builder.channels.keys().copied() {
            let exchange_tx = self.channels
                .entry(exchange)
                .or_default()
                .tx
                .clone();

            exchange_txs.insert(exchange, exchange_tx);
        }

        self.futures.push(Box::pin(async move {
            builder
                .init()
                .await?
                .streams
                .into_iter()
                .for_each(|(exchange, mut exchange_rx)| {
                    // Remove the matching exchange_tx
                    let exchange_tx = exchange_txs
                        .remove(&exchange)
                        .expect("all exchange_txs should be present here");

                    tokio::spawn(async move {
                        while let Some(event) = exchange_rx.recv().await {
                            let _ = exchange_tx.send(Output::from(event));
                        }
                    });
                });

            Ok(())
        }));




        // // Each StreamBuilder ExchangeChannel<Kind::Event> sends data to a ExchangeChannel<Output>
        // for exchange in builder.channels.keys() {
        //
        //     // Add ExchangeChannel<Output> Entry for each StreamBuilder exchange present
        //     let exchange_tx = self.channels
        //         .entry(*exchange)
        //         .or_default()
        //         .tx
        //         .clone();
        //
        //     // Init Streams<Kind::Event> & send mapped Outputs to the new exchange_tx
        //     self.futures.push(Box::pin(async move {
        //         builder
        //             .init()
        //             .await?
        //             .streams
        //             .into_values()
        //             .for_each(|mut exchange_rx| {
        //                 tokio::spawn(async move {
        //                     while let Some(event) = exchange_rx.recv().await {
        //                         let _ = exchange_tx.send(Output::from(event));
        //                     }
        //                 });
        //             });
        //
        //         Ok(())
        //     }));
        // }

        self
    }

    pub async fn init(self) -> Result<Streams<Output>, DataError> {
        // Await Stream initialisation futures and ensure success
        futures::future::try_join_all(self.futures).await?;

        // Construct Streams using each ExchangeChannel receiver
        Ok(Streams {
            streams: self
                .channels
                .into_iter()
                .map(|(exchange, channel)| (exchange, channel.rx))
                .collect()
        })
    }
}