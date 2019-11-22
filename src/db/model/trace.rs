use std::fs::File;
use std::io::Read;
use std::io::Write;
use std::process::Stdio;

use base64_stream::ToBase64Reader;
use diesel::*;
use rayon::prelude::*;
use serde::*;
use tokio::prelude::*;
use uuid::Uuid;

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
        let envs =
            self.environment.iter().cloned().zip(self.values.iter().cloned()).collect::<Vec<(String, String)>>();
        let args = self.options.clone();
        tokio::spawn(script.and_then(move |x| {
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
                Ok(child) => {
                    {
                        let mut input = child.stdin.expect("unable to get input");
                        input.write(crate::config::global_config().root_password.as_bytes()).unwrap();
                        input.flush().unwrap();
                    }
                    let output =
                        child.stdout.expect("unable to get output");
                    let mut buffer = Vec::new();
                    let mut output = ToBase64Reader::new(output);
                    buffer.resize(crate::config::global_config().submit_chunk_size, 0_u8);
                    let mut k = 0;
                    loop {
                        k += 1;
                        match output.read(buffer.as_mut()) {
                            Ok(n) => if n == buffer.len() {
                                submit(x.clone(), &buffer[0..n], false, None, k);
                            } else {
                                let stderr = child.stderr.map(
                                    |mut x|
                                        {
                                            let mut b = String::new();
                                            x.read_to_string(&mut b).expect("failed to get stderr");
                                            b
                                        });
                                submit(x.clone(), &buffer[0..n], true, stderr, k);
                                println!("[INFO] all submissions of {} finished.", x);
                                break;
                            },
                            Err(e) => {
                                eprintln!("[ERROR] error encountered when running {}: {}", x, e);
                                break;
                            }
                        }
                    }
                    Ok(())
                }
                Err(e) => {
                    eprintln!("unable to spawn process: {}", e);
                    Err(())
                }
            }
        }));
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