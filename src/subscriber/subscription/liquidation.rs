use super::SubKind;
use crate::model::Liquidation;
use serde::{Deserialize, Serialize};

/// Todo:
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Deserialize, Serialize)]
pub struct Liquidations;

impl SubKind for Liquidations {
    type Event = Liquidation;
}
