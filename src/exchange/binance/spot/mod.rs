use crate::model::subscription::{SubKind, SubscriptionMap};

pub struct BinanceSpot<Kind>
where
    Kind: SubKind,
{
    pub subscription_map: SubscriptionMap<Kind>
}

