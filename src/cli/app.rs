use std::fs::File;
use std::io::{BufWriter, stdout, Write};

use clap::*;
use regex::Regex;

use crate::http_server::{PutTrace, StartTrace};

fn get_matches<'a>() -> ArgMatches<'a> {
    let values = vec!["STAP", "BPF"];
    App::new("lambda-endpoint")
        .subcommand(SubCommand::with_name("endpoint").about("start endpoint")
            .arg(Arg::with_name("config").short("c").long("config").value_name("CONFIG")
                .help("path to the configuration").required(true)))
        .subcommand(SubCommand::with_name("add").about("add trace")
            .arg(Arg::with_name("config").short("c").long("config").value_name("CONFIG")
                .help("path to the configuration").required(true))
            .arg(Arg::with_name("process").short("p").long("proc").value_name("PROC")
                .help("path to the process to trace").required(true))
            .arg(Arg::with_name("function").short("f").long("func").value_name("FUNC")
                .help("functions to trace, allow multiple").multiple(true).required(true))
            .arg(Arg::with_name("environment").short("e").long("env").value_name("ENV")
                .help("trace environment variable, in the form of '(ENV_NAME, env_value)'").multiple(true))
            .arg(Arg::with_name("option").short("o").long("opt").value_name("OPT")
                .help("options to be append when tracing").multiple(true)))
        .subcommand(SubCommand::with_name("delete").about("delete trace")
            .arg(Arg::with_name("config").short("c").long("config").value_name("CONFIG")
                .help("path to the configuration").required(true))
            .arg(Arg::with_name("id").short("i").long("i").value_name("ID")
                .help("id of the trace to be deleted").required(true).multiple(true)))
        .subcommand(SubCommand::with_name("get").about("get trace")
            .arg(Arg::with_name("config").short("c").long("config").value_name("CONFIG")
                .help("path to the configuration").required(true))
            .arg(Arg::with_name("id").short("i").long("i").value_name("ID")
                .help("id of the trace to be got").required(true)))
        .subcommand(SubCommand::with_name("list").about("list add traces")
            .arg(Arg::with_name("config").short("c").long("config").value_name("CONFIG")
                .help("path to the configuration").required(true)))
        .subcommand(SubCommand::with_name("run").about("run the given trace")
            .arg(Arg::with_name("config").short("c").long("config").value_name("CONFIG")
                .help("path to the configuration").required(true))
            .arg(Arg::with_name("id").short("i").long("i").value_name("ID")
                .help("id of the trace to be got").required(true))
            .arg(Arg::with_name("duration").short("d").long("duration").value_name("DURATION")
                .help("duration of tracing").required(true))
            .arg(Arg::with_name("type").short("t").long("type").possible_values(values.as_slice())
                .value_name("TYPE").help("the type of trace").required(true))
            .arg(Arg::with_name("output").short("o").long("out").value_name("OUTPUT")
                .help("output file, will choose stdout if not set")))
        .get_matches()
}

lazy_static! {
    static ref MATCHES : ArgMatches<'static> = get_matches();
    pub static ref SUB_COMMAND : (&'static str, &'static ArgMatches<'static>) = {
        let (a, b) = MATCHES.subcommand();
        if b.is_none() {
            eprintln!("you must enter a subcommand, use --help to see details");
            std::process::exit(1);
        }
        (a, b.unwrap())
    };
}

fn init_config() -> &'static str {
    SUB_COMMAND.1.value_of("config").unwrap()
}

pub fn get_id() -> i32 {
    match SUB_COMMAND.1.value_of("id").and_then(|x| x.parse().ok()) {
        Some(t) => t,
        None => {
            eprintln!("invalid duration");
            std::process::exit(1)
        }
    }
}

pub fn get_ids() -> Vec<i32> {
    SUB_COMMAND.1.values_of("id").map(|id| {
        let mut u = Vec::new();
        id.for_each(|x| {
            if let Ok(i) = x.parse() {
                u.push(i)
            }
        });
        u
    }).unwrap_or(Vec::new())
}

pub fn get_task() -> StartTrace {
    if let Some(t) = SUB_COMMAND.1.value_of("duration").and_then(|x| x.parse().ok()) {
        StartTrace {
            trace_type: SUB_COMMAND.1.value_of("type").unwrap().to_string(),
            trace_id: get_id(),
            lasting: t,
        }
    } else {
        eprintln!("invalid duration");
        std::process::exit(1);
    }
}

pub fn get_stream() -> Box<dyn Write> {
    if let Some(path) = SUB_COMMAND.1.value_of("output") {
        let file = match File::create(path) {
            Ok(f) => f,
            Err(e) => {
                eprintln!("[ERROR] {}", e);
                std::process::exit(1);
            }
        };
        box BufWriter::new(file)
    } else {
        box BufWriter::new(stdout())
    }
}

fn get_multiple(name: &str) -> Vec<String> {
    SUB_COMMAND.1.values_of(name)
        .map(|x| x.map(|x| x.to_string()).collect())
        .unwrap_or(Vec::new())
}

fn get_env() -> (Vec<String>, Vec<String>) {
    let env = get_multiple("environment");
    let regex = r"\([\s]*([\da-zA-Z]+)[\s]*,[\s]*([\da-zA-Z]+)[\s]*\)";
    let rg = Regex::new(regex).unwrap();
    let mut a = Vec::new();
    let mut b = Vec::new();
    for i in env {
        if rg.is_match(i.as_str()) {
            let data = rg.captures(i.as_str()).take().unwrap();
            let (s, t) = (data.get(1).unwrap().as_str().to_string(),
                          data.get(2).unwrap().as_str().to_string());
            a.push(s);
            b.push(t);
        }
    }
    (a, b)
}

pub fn get_trace() -> PutTrace {
    let (a, b) = get_env();
    PutTrace {
        process: SUB_COMMAND.1.value_of("process").unwrap().to_string(),
        function_list: get_multiple("function"),
        environment: a,
        values: b,
        options: get_multiple("option"),
    }
}

lazy_static! {
    static ref CONFIG : &'static str = init_config();
}

pub fn config() -> &'static str {
    *CONFIG
}