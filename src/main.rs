#![feature(box_syntax)]
#[macro_use]
extern crate diesel;
#[macro_use]
extern crate gotham_derive;
#[macro_use]
extern crate lazy_static;

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


fn main() {
    notice();
    gotham::start(config::address(), http_server::router())
}



