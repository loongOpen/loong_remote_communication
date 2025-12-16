use serde::{Deserialize, Serialize};
use strum::{AsRefStr, Display, EnumString};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Display, EnumString, AsRefStr)]
pub enum SignalType {
    Offer,
    Answer,
    Candidate,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SignalPayload {
    pub from_id: String,
    pub payload: String,
    pub signal_type: SignalType,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Display, EnumString, AsRefStr)]
#[strum(serialize_all = "lowercase")]
pub enum PeerStatus {
    Online,
    Offline,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, AsRefStr)]
#[strum(serialize_all = "lowercase")]
pub enum SignalRole {
    Caller,
    Callee,
}
