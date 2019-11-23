use std::fs::File;
use std::io::Read;
use std::io::Write;
use std::process::{Child, ChildStderr, ChildStdin, ChildStdout, Stdio};
use std::sync::Arc;

use base64_stream::ToBase64Reader;
use chrono::Utc;
use diesel::*;
use rayon::prelude::*;
use serde::*;
use tokio::prelude::*;
use tokio::spawn;
use uuid::Uuid;

use crate::endpoint::{remove_running, RunningTrace};
use crate::http_client::submit;

#[derive(Queryable, Debug, Serialize, Deserialize)]
pub struct Trace {
    pub id: i32,
    pub process: String,
    pub function_list: Vec<String>,
    pub environment: Vec<String>,
    pub values: Vec<String>,
    pub options: Vec<String>,
}

fn submit_step(mut k: usize, mut stdout: ChildStdout, mut stderr: ChildStderr, name: String, mut buffer: Vec<u8>) {
    k += 1;
    match stdout.read(buffer.as_mut()) {
        Ok(n) => if n == buffer.len() {
            submit(name.clone(), &buffer[0..n], false, None, k);
            tokio::spawn(futures::future::lazy(move || Ok(
                submit_step(k, stdout, stderr, name, buffer))));
        } else {
            let stderr = Some({
                let mut b = String::new();
                stderr.read_to_string(&mut b).expect("failed to get stderr");
                b
            });
            submit(name.clone(), &buffer[0..n], true, stderr, k);
            println!("[INFO] all submissions of {} finished.", name);
            remove_running(name.as_str());
        },
        Err(e) => {
            eprintln!("[ERROR] error encountered when running {}: {}", name, e);
            remove_running(name.as_str());
        }
    }
}
macro_rules! template {
    ("STAP") => {
r#"
probe process("{}").function("{}").call {{
    printf("probe: %s", ppfunc());
    print_usyms(ucallers(5));
}}
"#
};
    ("BPF") => {
    r#"uprobe:{}:{} {{ printf("probe: %s\n%s\n", probe, ustack(perf, 5)); }}"#
    };
}

fn ending(s: &str, t: usize) -> String {
    match s {
        "STAP" =>
            format!("probe timer.s({}) {{exit(); }}\n", t),
        "BPF" =>
            format!("interval:s:({}) {{ exit(); }}\n", t),
        _ => unreachable!()
    }
}


impl Trace {
    pub fn to_content_stap(&self) -> String {
        self.function_list.par_iter().map(|x| {
            format!(template!("STAP"), self.process, x)
        }).reduce_with(|mut x, y| {
            x.push_str(y.as_str());
            x
        }).unwrap()
    }
    pub fn to_content_bpf(&self) -> String {
        self.function_list.par_iter().map(|x| {
            format!(template!("BPF"), self.process, x)
        }).reduce_with(|mut x, y| {
            x.push_str(y.as_str());
            x
        }).unwrap()
    }
    pub fn to_file_stap(&self, duration: usize) -> std::io::Result<String> {
        let content = self.to_content_stap();
        let name = format!("/tmp/{}.stap", Uuid::new_v4());
        let mut file = File::create(name.as_str())?;
        file.write(content.as_bytes())?;
        file.write(ending("STAP", duration).as_bytes())?;
        file.flush()?;
        Ok(name)
    }
    pub fn to_file_bpf(&self, duration: usize) -> std::io::Result<String> {
        let content = self.to_content_bpf();
        let name = format!("/tmp/{}.bpf", Uuid::new_v4());
        let mut file = File::create(name.as_str())?;
        file.write(content.as_bytes())?;
        file.write(ending("BPF", duration).as_bytes())?;
        file.flush()?;
        Ok(name)
    }

    pub fn run(&self, duration: usize, t: &str) -> String {
        let mut name = String::new();
        let script = if t == "STAP" {
            self.to_file_stap(duration)
                .map(|x| {
                    name.clone_from(&x);
                    x
                }).map_err(|x|
                eprintln!("failed to generate stap file: {}", x)
            ).into_future()
        } else {
            self.to_file_bpf(duration)
                .map(|x| {
                    name.clone_from(&x);
                    x
                }).map_err(|x|
                eprintln!("failed to generate stap file: {}", x)
            ).into_future()
        };
        let flag = t == "STAP";
        let id = self.id;
        let _name = name.clone();
        let envs =
            self.environment.iter().cloned().zip(self.values.iter().cloned()).collect::<Vec<(String, String)>>();
        let args = self.options.clone();
        let f = script.and_then(move |x| {
            crate::http_client::submit_start(x.clone());
            let child = std::process::Command::new("sudo")
                .arg("-S")
                .arg(if flag { crate::config::global_config().stap_path.as_str() } else { crate::config::global_config().stap_path.as_str() })
                .arg(x.as_str())
                .args(args)
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .stdin(Stdio::piped())
                .envs(envs)
                .spawn();
            match child {
                Ok(mut child) => {
                    {
                        let mut input = child.stdin.take().expect("unable to get input");
                        input.write(crate::config::global_config().root_password.as_bytes()).unwrap();
                        input.flush().unwrap();
                    }
                    let output =
                        child.stdout.take().expect("unable to get output");
                    let stderr =
                        child.stderr.take().expect("unable to get output");
                    let rt = RunningTrace {
                        start_time: Utc::now(),
                        trace_id: id,
                        child,
                    };
                    crate::endpoint::put_running(_name.as_str(), rt);
                    let mut buffer = Vec::new();

                    buffer.resize(crate::config::global_config().submit_chunk_size, 0_u8);
                    submit_step(0, output, stderr, _name, buffer);
                    Ok(())
                }
                Err(e) => {
                    eprintln!("unable to spawn process: {}", e);
                    Err(())
                }
            }
        });
        tokio::spawn(f);

        name
    }
}

#[cfg(test)]
mod test {
    #[test]
    fn test_to_content() {
        use std::fs::File;
        use std::io::Write;
        use crate::db::schema::trace::traces::dsl::*;
        use diesel::prelude::*;
        use super::Trace;
        let conn = crate::db::connection::get_conn();
        let res = traces.load::<Trace>(&*conn).unwrap();
        let mut file = File::create("/tmp/cargo_test").unwrap();
        for i in res {
            writeln!(file, "{}", i.to_content_stap()).unwrap();
        }
    }

    #[test]
    fn test_to_file() {
        use crate::db::schema::trace::traces::dsl::*;
        use diesel::prelude::*;
        use super::Trace;
        let conn = crate::db::connection::get_conn();
        let res = traces.load::<Trace>(&*conn).unwrap();
        for i in res {
            i.to_file_stap(5).unwrap();
        }
    }

    #[test]
    fn test_submit() {
        use futures::future;
        use crate::db::schema::trace::traces::dsl::*;
        use diesel::prelude::*;
        use super::Trace;
        let conn = crate::db::connection::get_conn();
        let res = traces.load::<Trace>(&*conn).unwrap();
        for i in res {
            tokio::run(future::lazy(move || {
                i.run(1, "STAP");
                Ok(())
            }))
        }
    }
}