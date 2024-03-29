use std::fs::File;
use std::io::{ErrorKind, Read};
use std::process::exit;

use serde::*;

use crate::cli::config;

#[derive(Debug, Serialize, Deserialize)]
pub struct DataBaseConfig {
    pub address: String,
    pub port: u16,
    pub username: String,
    pub password: String,
    pub database: String
}
#[derive(Debug, Serialize, Deserialize)]
pub struct GlobalConfig {
    pub root_password : String,
    pub bpf_path : String,
    pub stap_path : String,
    pub submit_chunk_size: usize,
    pub platform_url : String,
    pub secret: String,
    pub endpoint_uuid: String,
    pub listen_address: String,
    pub listen_port: u16,
    pub database_config: DataBaseConfig
}

fn init_config() -> GlobalConfig {
    let config = config();
    let mut buffer = String::new();
    match File::open(config)
        .and_then(|mut x|x.read_to_string(&mut buffer))
        .and_then(|_| toml::from_str(buffer.as_str())
            .map_err(|x| std::io::Error::new(ErrorKind::Other, x))) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("cannot load config file: {}", e);
            exit(2)
        }
    }
}

lazy_static!{
    static ref GLOBAL : GlobalConfig = init_config();
    static ref ADDR : String = format!("{}:{}", GLOBAL.listen_address, GLOBAL.listen_port);
}

pub fn global_config() -> &'static GlobalConfig {
    &*GLOBAL
}

pub fn address() -> &'static str {
    ADDR.as_str()
}