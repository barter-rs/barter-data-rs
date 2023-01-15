use self::builder::StreamBuilder;
use crate::{event::Market, exchange::ExchangeId, subscription::SubKind};
use std::collections::HashMap;
use tokio::sync::mpsc;
use tokio_stream::{wrappers::UnboundedReceiverStream, StreamMap};

/// Defines the [`StreamBuilder`](builder::StreamBuilder) API for initialising
/// [`MarketStream`](super::MarketStream) [`Streams`].
pub mod builder;

/// Central consumer loop functionality used by the [`StreamBuilder`](builder::StreamBuilder) to
/// to drive a re-connecting [`MarketStream`](super::MarketStream).
pub mod consumer;

/// Collection of exchange [`Market<Event>`](Market) streams for a specific [`SubKind`].
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

    /// Remove an exchange [`Market<Event>`](Market) [`mpsc::UnboundedReceiver`] from the
    /// [`Streams`] `HashMap`.
    pub fn select(
        &mut self,
        exchange: ExchangeId,
    ) -> Option<mpsc::UnboundedReceiver<Market<Kind::Event>>> {
        self.streams.remove(&exchange)
    }

    /// Join all exchange [`Market<Event>`](Market) [`mpsc::UnboundedReceiver`] streams into a
    /// unified [`mpsc::UnboundedReceiver`].
    pub async fn join(self) -> mpsc::UnboundedReceiver<Market<Kind::Event>>
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

    /// Join all exchange [`Market<Event>`](Market) [`mpsc::UnboundedReceiver`] streams into a
    /// unified [`StreamMap`].
    pub async fn join_map(
        self,
    ) -> StreamMap<ExchangeId, UnboundedReceiverStream<Market<Kind::Event>>> {
        self.streams
            .into_iter()
            .fold(StreamMap::new(), |mut map, (exchange, rx)| {
                map.insert(exchange, UnboundedReceiverStream::new(rx));
                map
            })
    }
}
