use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SignalRole {
    Caller,
    Callee,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MqttConfig {
    pub host: String,
    pub port: u16,
    pub username: Option<String>,
    pub password: Option<String>,
    pub keep_alive: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignalMessage {
    pub from_id: String,
    pub to_id: String,
    pub payload: String,  // JSON 或 SDP candidate 等
    pub msg_type: String, // offer / answer / candidate / status
}
