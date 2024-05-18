use barter_integration::model::instrument::kind::InstrumentKind;
use barter_integration::model::instrument::Instrument;
use derive_more::Display;
use serde::{Deserialize, Serialize};
use std::fmt::Debug;

/// Concise unique identifier for an instrument. Used to key
/// [MarketEvents](crate::event::MarketEvent) in a memory efficient way.
#[derive(
    Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Deserialize, Serialize, Display,
)]
pub struct InstrumentId(pub u64);

/// Instrument related data that defines an associated unique `Id`.
///
/// Verbose `InstrumentData` is often used to subscribe to market data feeds, but it's unique `Id`
/// can then be used to key consumed [MarketEvents](crate::event::MarketEvent), significantly reducing
/// duplication in the case of complex instruments (eg/ options).
pub trait InstrumentData
where
    Self: Clone + Debug + Send + Sync,
{
    type Id: Debug + Clone + Send + Sync;
    fn id(&self) -> &Self::Id;
}

impl InstrumentData for Instrument {
    type Id = Self;

    fn id(&self) -> &Self::Id {
        self
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Deserialize, Serialize)]
pub struct MarketInstrumentData {
    pub id: InstrumentId,
    pub name_exchange: String,
    pub kind: InstrumentKind,
}

impl InstrumentData for MarketInstrumentData {
    type Id = InstrumentId;

    fn id(&self) -> &Self::Id {
        &self.id
    }
}
