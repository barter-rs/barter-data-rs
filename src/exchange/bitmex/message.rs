use crate::{exchange::bitmex::trade::BitmexTradePayload, Identifier};
use barter_integration::model::SubscriptionId;
use serde::{Deserialize, Serialize};

/// ### Raw Payload Examples
/// See docs: <https://www.bitmex.com/app/wsAPI#Response-Format>
/// #### Base payload
///```json
/// {
///     "table": string,
///     "action": 'update' | 'insert' | 'delete',
///     "data": [{}],
/// }
/// ```
/// #### Partial payload
///```json
/// {
///     "table": string,
///     "action": 'partial',
///     "data": [{}],
///     "keys": [],
/// }
/// ```
/// #### Trade payload
/// ```json
/// {
///     "table": "trade",
///     "action": "insert",
///     "data": [
///         {
///             "timestamp": "2023-02-18T09:27:59.701Z",
///             "symbol": "XBTUSD",
///             "side": "Sell",
///             "size": 200,
///             "price": 24564.5,
///             "tickDirection": "MinusTick",
///             "trdMatchID": "31e50cb7-e005-a44e-f354-86e88dff52eb",
///             "grossValue": 814184,
///             "homeNotional": 0.00814184,
///             "foreignNotional": 200,
///             "trdType": "Regular"
///         }
///     ]
/// }
///```
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Debug, Deserialize, Serialize)]
pub struct BitmexMessage<T> {
    pub table: String,
    pub action: String,
    pub data: Vec<T>,
}

impl Identifier<Option<SubscriptionId>> for BitmexTradePayload {
    fn id(&self) -> Option<SubscriptionId> {
        let subscription_id = format!("{}|{}", self.table, self.data.first().unwrap().symbol);
        Some(SubscriptionId(subscription_id))
    }
}
