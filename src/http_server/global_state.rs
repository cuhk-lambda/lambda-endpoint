use std::sync::Arc;

use chrono::{DateTime, Utc};
use hashbrown::HashMap;
use parking_lot::RwLock;

use crate::db::connection::get_conn;
use crate::endpoint::RunningTrace;

#[derive(Clone, StateData)]
pub struct GlobalState {
    pub start_time: DateTime<Utc>
}

impl std::panic::RefUnwindSafe for GlobalState {}

impl GlobalState {
    pub fn new() -> Self {
        get_conn();
        println!("[INFO] database connected");
        let time = Utc::now();
        println!("[INFO] http service is now online: {:#}", time);
        GlobalState {
            start_time: time
        }
    }
}
