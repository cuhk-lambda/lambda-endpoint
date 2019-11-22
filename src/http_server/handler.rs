use chrono::Utc;
use futures::prelude::*;
use gotham::handler::HandlerFuture;
use gotham::helpers::http::response::*;
use gotham::state::{FromState, State};
use hyper::{Body, HeaderMap, Response, StatusCode};

use crate::config::global_config;
use crate::diesel::prelude::*;
use crate::endpoint::*;
use crate::http_server::global_state::GlobalState;
use crate::http_server::reply::{ErrorReply, StartTraceReply, StateReply};

use super::requests::*;

fn verify_request(state: &State) -> Result<bool, String> {
    let headers = HeaderMap::borrow_from(state);
    headers.get("Authorization")
        .ok_or("no header".to_string())
        .and_then(|x| x
            .to_str()
            .map_err(|e| e.to_string()))
        .map(|x| x.split("$argon2").collect::<Vec<&str>>())
        .map(|x| {
            if x.len() == 2 {
                let y = String::from("$argon2") + x[1];
                x[0] == global_config().endpoint_uuid && verify(y.as_str())
            } else { false }
        })
}

pub fn heartbeat(state: State) -> (State, Response<Body>) {
    let verification = verify_request(&state);
    let temp = verification.map(|x|
        if x {
            let time = chrono::Utc::now();
            let reply = super::reply::HeartbeatReply { status: "alive".to_string(), time };
            let body = Body::from(serde_json::to_string(&reply).unwrap());
            create_response(&state, StatusCode::OK, mime::APPLICATION_JSON, body)
        } else {
            create_empty_response(&state, StatusCode::UNAUTHORIZED)
        }
    );
    let response = temp.unwrap_or_else(|x| {
        let body = Body::from(x);
        create_response(&state, StatusCode::UNAUTHORIZED, mime::TEXT_PLAIN, body)
    });
    (state, response)
}

pub fn endpoint_state(state: State) -> (State, Response<Body>) {
    let gstate = GlobalState::borrow_from(&state);
    let running_time: time::Duration = Utc::now() - gstate.start_time;
    let running_time = std::time::Duration::new(running_time.num_seconds() as u64, 0);
    let reply = serde_json::to_string(&StateReply {
        uuid: global_config().endpoint_uuid.as_str(),
        start_time: gstate.start_time,
        running_time,
    }).unwrap();
    let body = Body::from(reply);
    let res = create_response(&state, StatusCode::OK, mime::APPLICATION_JSON, body);
    (state, res)
}

pub fn trace_list(state: State) -> (State, Response<Body>) {
    use crate::db::schema::trace::traces::dsl::*;
    use crate::db::model::trace::*;
    match verify_request(&state) {
        Ok(true) => {
            let conn = crate::db::connection::get_conn();
            let result = traces
                .load::<Trace>(&*conn).expect("failed to load trace");
            let json = serde_json::to_string(&result).unwrap();
            let body = Body::from(json);
            let res = create_response(&state, StatusCode::OK, mime::APPLICATION_JSON, body);
            (state, res)
        }
        Ok(false) => {
            let json = serde_json::to_string(&ErrorReply { error: "unauthorized".to_string() }).unwrap();
            let body = Body::from(json);
            let res = create_response(&state, StatusCode::UNAUTHORIZED, mime::APPLICATION_JSON, body);
            (state, res)
        }
        Err(e) => {
            let json = ErrorReply { error: format!("{}", e) };
            let body = Body::from(serde_json::to_string(&json).unwrap());
            let res = create_response(&state, StatusCode::OK, mime::APPLICATION_JSON, body);
            (state, res)
        }
    }
}

pub fn start_trace(mut state: State) -> Box<HandlerFuture> {
    use crate::db::schema::trace::traces::dsl::*;
    use crate::db::model::trace::*;
    let verify = verify_request(&state);
    let body = Body::take_from(&mut state);
    let f = body.concat2().then(move |real| match real {
        Ok(x) => {
            match verify {
                Ok(true) => {
                    let json =
                        simd_json::serde::from_slice::<StartTrace>(x.to_vec().as_mut_slice());
                    let reply = match json {
                        Ok(e) => {
                            let conn = crate::db::connection::get_conn();
                            let result = traces.filter(id.eq(e.trace_id))
                                .limit(1)
                                .load::<Trace>(&*conn).expect("failed to load trace");

                            if let Some(trace) = result.first() {
                                let script = trace.run(e.lasting as _, e.trace_type.as_str());
                                serde_json::to_string(&StartTraceReply { file_path: script })
                            } else {
                                serde_json::to_string(&ErrorReply { error: "no such trace".to_string() })
                            }
                        }
                        Err(k) => {
                            serde_json::to_string(&ErrorReply { error: format!("error: {}", k) })
                        }
                    };
                    let body = Body::from(reply.unwrap());
                    let res = create_response(&state, StatusCode::OK, mime::APPLICATION_JSON, body);
                    Ok((state, res))
                }
                Ok(false) => {
                    let json = serde_json::to_string(&ErrorReply { error: "unauthorized".to_string() }).unwrap();
                    let body = Body::from(json);
                    let res = create_response(&state, StatusCode::UNAUTHORIZED, mime::APPLICATION_JSON, body);
                    Ok((state, res))
                }
                Err(e) => {
                    let json = serde_json::to_string(&ErrorReply { error: e }).unwrap();
                    let body = Body::from(json);
                    let res = create_response(&state, StatusCode::UNAUTHORIZED, mime::APPLICATION_JSON, body);
                    Ok((state, res))
                }
            }
        }
        Err(e) => {
            let json = ErrorReply { error: format!("{}", e) };
            let body = Body::from(serde_json::to_string(&json).unwrap());
            let res = create_response(&state, StatusCode::OK, mime::APPLICATION_JSON, body);
            Ok((state, res))
        }
    });

    Box::new(f)
}