use super::SubKind;
use crate::model::PublicTrade;
use serde::{Deserialize, Serialize};

/// Todo:
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Deserialize, Serialize)]
pub struct PublicTrades;

impl SubKind for PublicTrades {
    type Event = PublicTrade;
}
