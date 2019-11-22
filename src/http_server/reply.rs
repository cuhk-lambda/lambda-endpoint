use std::time::Duration;

use chrono::prelude::*;
use serde::*;

#[derive(Debug, Serialize, Deserialize)]
pub struct HeartbeatReply {
    pub status: String,
    pub time: DateTime<Utc>
}

#[derive(Debug, Serialize, Deserialize)]
pub struct StateReply {
    pub uuid: &'static str,
    pub start_time: DateTime<Utc>,
    pub running_time: Duration,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ErrorReply {
    pub error: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct StartTraceReply {
    pub file_path: String
}
