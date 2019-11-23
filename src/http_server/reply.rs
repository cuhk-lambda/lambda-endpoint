use std::time::Duration;

use chrono::prelude::*;
use serde::*;

use crate::db::model::trace::Trace;
use crate::diesel::prelude::*;

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

#[derive(Debug, Serialize, Deserialize)]
pub struct RunningTraceReply {
    pub file_path: String,
    pub start_time: DateTime<Utc>,
    pub content: Trace,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct KillReply {
    pub killed: bool
}

impl RunningTraceReply {
    pub(crate) fn new(file_path: String, start_time: DateTime<Utc>, t_id: i32) -> Self {
        use crate::db::schema::trace::traces::dsl::*;
        use crate::db::model::trace::*;
        let conn = crate::db::connection::get_conn();
        let content = traces.filter(id.eq(t_id))
            .limit(1)
            .load::<Trace>(&*conn).expect("failed to load trace").pop().unwrap();
        RunningTraceReply {
            file_path,
            start_time,
            content,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DeleteReply {
    pub deleted: usize
}
