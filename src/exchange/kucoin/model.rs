use serde::{Deserialize, Serialize};


/// Container for multiple message types 
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DefaultMsg {
    /// Unique message id
    pub id: String,
    /// Type of this default message
    pub r#type: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
#[serde(untagged)]
pub enum KucoinMessage {
    /// Default message type
    DefaultMsgEvent(DefaultMsg),
}