use super::SubKind;
use crate::model::PublicTrade;

/// Todo:
#[derive(Debug, Clone)]
pub struct PublicTrades;

impl SubKind for PublicTrades {
    type Event = PublicTrade;
}
