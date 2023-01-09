use self::builder::StreamBuilder;
use crate::{event::Market, exchange::ExchangeId, subscription::SubKind};
use std::collections::HashMap;
use tokio::sync::mpsc;

/// Todo:
pub mod builder;
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
}
