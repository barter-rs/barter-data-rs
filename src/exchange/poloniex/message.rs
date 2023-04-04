use crate::{exchange::poloniex::trade::PoloniexTrade, Identifier};
use barter_integration::model::SubscriptionId;
use serde::{Deserialize, Serialize};

///Poloniex real-time trade websocket messsage.
///
/// ### Raw Payload Examples
/// See docs: <https://docs.poloniex.com/#public-channels-market-data-trades>
/// ```json
///{
///    "channel": "trades",
///    "data": [{
///      "symbol": "BTC_USDT",
///      "amount": "70", 
///      "takerSide": "buy",
///      "quantity": "4",
///      "createTime": 1648059516810,
///      "price": "104", 
///      "id": 1648059516810, 
///      "ts": 1648059516832
///    }]
///}
/// ```

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Debug, Deserialize, Serialize)]
pub struct PoloniexMessage<T> {
    pub channel: String,
    pub data: Vec<T>,
}

impl Identifier<Option<SubscriptionId>> for PoloniexTrade {
    fn id(&self) -> Option<SubscriptionId> {
        self.data
            .first()
            .map(|trade| SubscriptionId(format!("{}|{}", self.channel, trade.symbol)))
            .or(None)
    }
}
