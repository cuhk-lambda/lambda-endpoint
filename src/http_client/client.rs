use hyper::StatusCode;
use serde::*;

use crate::endpoint::authorization;

#[derive(Serialize, Deserialize, Debug)]
struct SubmitInfo {
    trace: String,
    status: String,
    body: Option<String>,
    stderr: Option<String>,
    no: usize
}

pub fn submit_start(x: String) {
    let info = serde_json::to_string(
        &SubmitInfo { trace: x, status: "start".to_string(), body: None, stderr: None, no: 0 }
    ).unwrap();
    let client = reqwest::Client::new();
    let url = if cfg!(test) { "http://httpbin.org/post".to_string() } else { crate::config::global_config().platform_url.clone() + "/submit" };
    match client
        .post(url.as_str())
        .header("Authorization", authorization())
        .body(info).send() {
        Ok(req) if req.status() == StatusCode::OK =>
            println!("[INFO] submit successfully started"),
        _ =>
            eprintln!("[ERROR] server failed to response start request")
    };
}

pub fn submit(x: String, body: &[u8], is_end: bool, stderr: Option<String>, no: usize) {
    let body = Some(String::from_utf8_lossy(body).to_string());
    let info = serde_json::to_string(
        &SubmitInfo {
            trace: x.clone(),
            status:
            if is_end { "finished".to_string() } else { "WIP".to_string() },
            body,
            stderr,
            no,
        }
    ).unwrap();
    let client = reqwest::Client::new();
    let url = if cfg!(test) {
        "http://httpbin.org/post".to_string()
    } else {
        crate::config::global_config().platform_url.clone() + "/submit"
    };
    match client
        .post(url.as_str())
        .header("Authorization", authorization())
        .body(info).send() {
        Ok(req) if req.status() == StatusCode::OK =>
            println!("[INFO] submit {} of {} finished", no, x),
        _ =>
            eprintln!("[ERROR] server failed to response submit request")
    };
}

#[test]
fn test_start() {
    let x = "123".to_string();
    let info = serde_json::to_string(
        &SubmitInfo { trace: x, status: "start".to_string(), body: None, stderr: None, no: 0 }
    ).unwrap();
    let client = reqwest::Client::new();
    let url = "http://httpbin.org/post";
    match client
        .post(url)
        .header("Authorization", authorization())
        .body(info).send() {
        Ok(req) if req.status() == StatusCode::OK =>
            println!("[INFO] submit successfully started"),
        _ =>
            eprintln!("[ERROR] server failed to response start request")
    };
}