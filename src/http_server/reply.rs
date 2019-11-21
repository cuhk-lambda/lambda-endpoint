use chrono::prelude::*;
use serde::*;
#[derive(Debug, Serialize, Deserialize)]
pub struct HeartbeatReply {
    pub status: String,
    pub time: DateTime<Utc>
}

#[derive(Debug, Serialize, Deserialize)]
pub struct StartTraceReply {
    pub thread: String,
    pub file_path: String
}