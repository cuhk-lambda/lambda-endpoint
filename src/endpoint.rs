use std::cell::RefCell;
use std::process::{Child, ChildStdin};
use std::sync::Arc;

use argon2::Config;
use chrono::{DateTime, Utc};
use crypto_api_osrandom::OsRandom;
use hashbrown::HashMap;
use parking_lot::RwLock;
use serde::*;

use crate::config::*;

pub struct RunningTrace {
    pub start_time: DateTime<Utc>,
    pub trace_id: i32,
    pub child: Child
}

impl RunningTrace {
    pub fn kill(&mut self) {
        self.child.kill().unwrap_or(());
    }
}

lazy_static! {
    pub static ref RUNNING:  RwLock<HashMap<String, RunningTrace>> = RwLock::new(HashMap::new());
}

pub fn put_running(x: &str, trace: RunningTrace) {
    let mut writer = RUNNING.write();
    writer.insert(x.to_string(), trace);
    drop(writer)
}

pub fn remove_running(x: &str) {
    let mut writer = RUNNING.write();
    writer.remove(x);
    drop(writer)
}

pub fn hashed_secret() -> String {
    let mut gen = OsRandom::secure_rng();
    let mut salt  = Vec::new();
    salt.resize(16, 0);
    gen.random(salt.as_mut_slice()).expect("unable to gen salt");
    let password = global_config().secret.as_bytes();
    let config = Config::default();
    argon2::hash_encoded(password, salt.as_slice(), &config).expect("unable to hash secret")
}

pub fn verify(encoded: &str) -> bool {
    match argon2::verify_encoded(encoded, global_config().secret.as_bytes()) {
        Ok(true) => true,
        _ => false
    }
}

pub fn authorization() -> String {
    global_config().endpoint_uuid.clone() + hashed_secret().as_str()
}


#[test]
fn basic_hash() {
    let a = hashed_secret();
    let b = argon2::verify_encoded(a.as_str(), global_config().secret.as_bytes());
    assert!(b.unwrap());
    assert!(verify(a.as_str()))
}