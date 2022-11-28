use super::SubKind;
use crate::model::Liquidation;

/// Todo:
#[derive(Debug, Clone)]
pub struct Liquidations;

impl SubKind for Liquidations {
    type Event = Liquidation;
}
