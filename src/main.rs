#![feature(box_syntax)]
#[macro_use]
extern crate diesel;
#[macro_use]
extern crate gotham_derive;
#[macro_use]
extern crate lazy_static;

use std::io::{Read, Write};
use std::process::Stdio;

use crate::cli::{get_id, get_ids, get_stream, get_task, get_trace};
use crate::cli::app::SUB_COMMAND;
use crate::endpoint::hashed_secret;

#[global_allocator]
static ALLOC: jemallocator::Jemalloc = jemallocator::Jemalloc;

mod config;
mod cli;
mod db;
mod endpoint;
mod http_server;
mod http_client;

fn notice() {
    println!("Dev Hash: {}", hashed_secret());
    println!("Listening for requests at http://{}", config::address());
    println!("loaded config: {}", cli::config());
    println!("uuid: {}", config::global_config().endpoint_uuid);
    println!("===============================================================");
}

mod db_prelude {
    pub use diesel::prelude::*;

    pub use crate::db::model::trace::*;
    pub use crate::db::model::trace::Trace;
    pub use crate::db::schema::trace::traces;
    pub use crate::db::schema::trace::traces::dsl::*;
}

fn main() {
    match SUB_COMMAND.0 {
        "endpoint" => {
            notice();
            gotham::start(config::address(), http_server::router())
        }
        "add" => {
            use db_prelude::*;
            let trace = get_trace();
            let conn = crate::db::connection::get_conn();
            match diesel::insert_into(traces::table).values(&trace).get_result::<Trace>(&*conn) {
                Ok(res) => {
                    println!("[INFO] new trace put: {:#}", serde_json::to_string_pretty(&res).unwrap());
                }
                Err(m) => {
                    println!("[ERROR] {:#}", m);
                }
            }
        }
        "delete" => {
            use db_prelude::*;

            let conn = crate::db::connection::get_conn();
            let mut k = 0;
            for i in get_ids() {
                match diesel::delete(traces.filter(id.eq(i)))
                    .execute(&*conn)
                    {
                        Ok(e) => k += e,
                        Err(e) => eprintln!("[ERROR] {}", e)
                    }
            }
            println!("[INFO] deleted {} record(s)", k);
        }
        "list" => {
            use db_prelude::*;

            let conn = crate::db::connection::get_conn();
            let result = traces
                .load::<Trace>(&*conn);
            match result {
                Ok(res) => {
                    let json = serde_json::to_string_pretty(&res).unwrap();
                    println!("{:#}", json);
                }
                Err(e) => {
                    eprintln!("[ERROR] {}", e)
                }
            }
        }
        "get" => {
            use db_prelude::*;
            let conn = crate::db::connection::get_conn();
            let tid = get_id();
            let result = traces.filter(id.eq(tid)).first::<Trace>(&*conn);
            match result {
                Ok(res) => {
                    let json = serde_json::to_string_pretty(&res).unwrap();
                    println!("{:#}", json);
                }
                Err(e) => {
                    eprintln!("[ERROR] {}", e)
                }
            }
        }
        "run" => {
            use db_prelude::*;
            let conn = crate::db::connection::get_conn();
            let task = get_task();
            let mut stream = get_stream();
            let result: Result<Trace, _> = traces.filter(id.eq(task.trace_id)).first::<Trace>(&*conn);
            match result {
                Err(e) => {
                    eprintln!("[ERROR] {}", e)
                }
                Ok(res) => {
                    let mut name = String::new();
                    let script = if task.trace_type == "STAP" {
                        res.to_file_stap(task.lasting as _).map(|x| {
                            name.clone_from(&x);
                            x
                        })
                    } else {
                        res.to_file_bpf(task.lasting as _).map(|x| {
                            name.clone_from(&x);
                            x
                        })
                    };
                    let envs =
                        res.environment.iter().cloned().zip(res.values.iter().cloned()).collect::<Vec<(String, String)>>();
                    let args = res.options.clone();
                    let flag = task.trace_type == "STAP";
                    let res = script.and_then(|x| std::process::Command::new("sudo")
                        .arg("-S")
                        .arg(if flag { crate::config::global_config().stap_path.as_str() } else { crate::config::global_config().stap_path.as_str() })
                        .arg(x.as_str())
                        .args(args)
                        .stdout(Stdio::piped())
                        .stderr(Stdio::piped())
                        .stdin(Stdio::piped())
                        .envs(envs)
                        .spawn())
                        .and_then(move |mut child| {
                            {
                                let mut input = child.stdin.take().expect("unable to get input");
                                input.write(crate::config::global_config().root_password.as_bytes()).unwrap();
                                input.flush().unwrap();
                            }
                            let mut output =
                                child.stdout.take().expect("unable to get output");
                            let mut stderr =
                                child.stderr.take().expect("unable to get output");
                            std::io::copy(&mut output, stream.as_mut()).and_then(|_| {
                                let mut err = String::new();
                                let res = stderr.read_to_string(&mut err);
                                if !err.is_empty() { eprintln!("[ERROR] {}", err); }
                                res
                            }).map(|_| ())
                        });
                    match res {
                        Ok(()) => (),
                        Err(e) => {
                            eprintln!("[ERROR] {}", e)
                        }
                    }
                }
            }
        }
        _ => unreachable!()
    }
}



