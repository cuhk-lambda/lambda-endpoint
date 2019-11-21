use gotham::state::{State, FromState};
use gotham::helpers::http::response::*;
use hyper::{HeaderMap, Response, Body, StatusCode};
use crate::endpoint::*;
use crate::config::global_config;

fn verify_request(state: &State) -> Result<bool, String> {
    let headers = HeaderMap::borrow_from(state);
    headers.get("Authorization")
        .ok_or("no header".to_string())
        .and_then(|x|x
            .to_str()
            .map_err(|e|e.to_string()))
        .map(|x|x.split("$argon2").collect::<Vec<&str>>())
        .map(|x| {
            if x.len() == 2 {
                let y = String::from("$argon2") + x[1];
                x[0] == global_config().endpoint_uuid  && verify(y.as_str())
            }
            else { false }
        })
}

pub fn heartbeat(state: State) -> (State, Response<Body>) {
    let verification = verify_request(&state);
    let temp   = verification.map(|x|
        if x {
            let time = chrono::Utc::now();
            let reply = super::reply::HeartbeatReply { status: "alive".to_string(), time };
            let body = Body::from(serde_json::to_string(&reply).unwrap());
            create_response(&state, StatusCode::OK, mime::APPLICATION_JSON, body)
        } else {
            create_empty_response(&state, StatusCode::UNAUTHORIZED)
        }
    );
    let response = temp.unwrap_or_else( |x| {
        let body = Body::from(x);
        create_response(&state, StatusCode::UNAUTHORIZED, mime::TEXT_PLAIN, body)
    });
    (state, response)
}