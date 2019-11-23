use std::fmt::Display;

use chrono::Utc;
use futures::prelude::*;
use gotham::handler::HandlerFuture;
use gotham::helpers::http::response::*;
use gotham::state::{FromState, State};
use hyper::{Body, HeaderMap, Response, StatusCode};
use rayon::prelude::*;
use serde::Serialize;

use crate::config::global_config;
use crate::diesel::prelude::*;
use crate::endpoint::*;
use crate::http_server::global_state::GlobalState;
use crate::http_server::reply::{DeleteReply, ErrorReply, KillReply, RunningTraceReply, StartTraceReply, StateReply};

use super::requests::*;

fn verify_request(state: &State) -> Result<bool, String> {
    let headers = HeaderMap::borrow_from(state);
    headers.get("Authorization")
        .ok_or("no authorization header".to_string())
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

fn with_verification(state: State, todo: Box<dyn Fn(State) -> (State, Response<Body>)>) -> (State, Response<Body>) {
    let verification = verify_request(&state);
    match verification {
        Ok(true) => todo(state),
        Ok(false) => {
            let json = serde_json::to_string(&ErrorReply { error: "unauthorized".to_string() }).unwrap();
            let body = Body::from(json);
            let res = create_response(&state, StatusCode::UNAUTHORIZED, mime::APPLICATION_JSON, body);
            (state, res)
        }
        Err(e) => {
            let json = ErrorReply { error: format!("{}", e) };
            let body = Body::from(serde_json::to_string(&json).unwrap());
            let res = create_response(&state, StatusCode::BAD_REQUEST, mime::APPLICATION_JSON, body);
            (state, res)
        }
    }
}

fn with_verification_res<E>(state: State, todo: Box<dyn Fn(State) -> Result<(State, Response<Body>), E>>) -> Result<(State, Response<Body>), E> {
    let verification = verify_request(&state);
    match verification {
        Ok(true) => todo(state),
        Ok(false) => {
            let json = serde_json::to_string(&ErrorReply { error: "unauthorized".to_string() }).unwrap();
            let body = Body::from(json);
            let res = create_response(&state, StatusCode::UNAUTHORIZED, mime::APPLICATION_JSON, body);
            Ok((state, res))
        }
        Err(e) => {
            let json = ErrorReply { error: format!("{}", e) };
            let body = Body::from(serde_json::to_string(&json).unwrap());
            let res = create_response(&state, StatusCode::BAD_REQUEST, mime::APPLICATION_JSON, body);
            Ok((state, res))
        }
    }
}

fn to_err_response<E: Display>(state: State, e: E, code: StatusCode) -> (State, Response<Body>) {
    let json = ErrorReply { error: format!("{}", e) };
    let body = Body::from(serde_json::to_string(&json).unwrap());
    let res = create_response(&state, code, mime::APPLICATION_JSON, body);
    (state, res)
}

fn to_json_response<J: Serialize>(state: State, j: &J) -> (State, Response<Body>) {
    let json = serde_json::to_string(j).unwrap();
    let body = Body::from(json);
    let res = create_response(&state, StatusCode::OK, mime::APPLICATION_JSON, body);
    (state, res)
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
    with_verification(state, box |state| {
        let conn = crate::db::connection::get_conn();
        let result = traces
            .load::<Trace>(&*conn).expect("failed to load trace");
        let json = serde_json::to_string(&result).unwrap();
        let body = Body::from(json);
        let res = create_response(&state, StatusCode::OK, mime::APPLICATION_JSON, body);
        (state, res)
    })
}

pub fn running_traces(state: State) -> (State, Response<Body>) {
    with_verification(state, box |state| {
        let list = {
            let reader = crate::endpoint::RUNNING.read();
            reader.par_iter().map(|(path, i)| {
                RunningTraceReply::new(path.clone(), i.start_time, i.trace_id)
            }).collect::<Vec<_>>()
        };
        let json = serde_json::to_string(&list).unwrap();
        let body = Body::from(json);
        let res = create_response(&state, StatusCode::OK, mime::APPLICATION_JSON, body);
        (state, res)
    })
}

pub fn kill_trace(mut state: State) -> Box<HandlerFuture> {
    let body = Body::take_from(&mut state);
    let f = body.concat2().then(move |real| match real {
        Ok(x) => {
            with_verification_res(state, box move |state| {
                let json =
                    simd_json::serde::from_slice::<KillTrace>(x.to_vec().as_mut_slice());
                let reply = match json {
                    Ok(e) =>
                        {
                            let mut writer = crate::endpoint::RUNNING.write();
                            let res = match writer.get_mut(e.file_path.as_str()) {
                                Some(t) => {
                                    t.kill();
                                    serde_json::to_string(&KillReply { killed: true }).unwrap()
                                }
                                None => {
                                    serde_json::to_string(&ErrorReply { error: "no such process".to_string() }).unwrap()
                                }
                            };
                            writer.remove(e.file_path.as_str());
                            res
                        }
                    Err(e) =>
                        serde_json::to_string(&ErrorReply { error: format!("{}", e) }).unwrap()
                };
                let body = Body::from(reply);
                let res = create_response(&state, StatusCode::OK, mime::APPLICATION_JSON, body);
                Ok((state, res))
            })
        }
        Err(e) => {
            Ok(to_err_response(state, e, StatusCode::BAD_REQUEST))
        }
    });
    Box::new(f)
}

pub fn start_trace(mut state: State) -> Box<HandlerFuture> {
    use crate::db::schema::trace::traces::dsl::*;
    use crate::db::model::trace::*;
    let body = Body::take_from(&mut state);
    let f = body.concat2().then(move |real| match real {
        Ok(x) => {
            with_verification_res(state, box move |state| {
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
            })
        }
        Err(e) => {
            Ok(to_err_response(state, e, StatusCode::BAD_REQUEST))
        }
    });

    Box::new(f)
}

pub fn put_trace(mut state: State) -> Box<HandlerFuture> {
    let body = Body::take_from(&mut state);
    use crate::db::schema::trace::traces;
    use crate::db::model::trace::Trace;
    let f = body.concat2().then(|x| {
        with_verification_res(state, box move |state| match &x {
            Err(e) => Ok(to_err_response(state, e, StatusCode::BAD_REQUEST)),
            Ok(body) => {
                match simd_json::serde::from_slice::<PutTrace>(body.to_vec().as_mut_slice()) {
                    Ok(p) => {
                        if p.function_list.len() == 0 {
                            Ok(to_err_response(state, "empty function list", StatusCode::BAD_REQUEST))
                        } else if p.environment.len() != p.values.len() {
                            Ok(to_err_response(state, "wrong size of environment values", StatusCode::BAD_REQUEST))
                        } else {
                            let conn = crate::db::connection::get_conn();
                            match diesel::insert_into(traces::table).values(&p).get_result::<Trace>(&*conn) {
                                Ok(res) => {
                                    println!("[INFO] new trace put: {:#}", serde_json::to_string_pretty(&res).unwrap());
                                    Ok(to_json_response(state, &res))
                                },
                                Err(m) => Ok(to_err_response(state, m, StatusCode::BAD_REQUEST))
                            }
                        }
                    },
                    Err(e) => Ok(to_err_response(state, e, StatusCode::BAD_REQUEST))
                }
            }
        })
    });
    box f
}

pub fn delete_trace(mut state: State) -> Box<HandlerFuture> {
    let body = Body::take_from(&mut state);
    use crate::db::schema::trace::traces::dsl::*;
    let f = body.concat2().then(|x| {
        match x {
            Ok(body) => {
                match simd_json::serde::from_slice::<DeleteTrace>(body.to_vec().as_mut_slice()) {
                    Ok(del) => {
                        let conn = crate::db::connection::get_conn();
                        match diesel::delete(traces.filter(id.eq(del.trace_id)))
                            .execute(&*conn)
                            {
                                Ok(e) => Ok(to_json_response(state, &DeleteReply { deleted: e })),
                                Err(e) =>
                                    Ok(to_err_response(state, e, StatusCode::BAD_REQUEST))
                            }
                    },
                    Err(e) => Ok(to_err_response(state, e, StatusCode::BAD_REQUEST))
                }
            },
            Err(e) => Ok(to_err_response(state, e, StatusCode::BAD_REQUEST))
        }
    });
    box f
}