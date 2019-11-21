use serde::*;
use reqwest::Response;
use hyper::StatusCode;

#[derive(Serialize, Deserialize, Debug)]
struct SubmitInfo {
    trace: String,
    status: String
}

fn submit_start(x: String) {
    let info = serde_json::to_string(
        &SubmitInfo{trace: x, status: "start".to_string()}
    ).unwrap();
    let client = reqwest::Client::new();
    let url = crate::config::global_config().platform_url.clone() + "/submit";
    match client.post(url.as_str()).body(info).send() {
        Ok(req) if req.status() == StatusCode::OK =>
            println!("submit successfully started"),
        _ =>
            eprintln!("server failed to response start request")
    };
}