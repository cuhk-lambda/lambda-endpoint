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
    println!("loaded config: {}", cli::config());
    println!("uuid: {}", config::global_config().endpoint_uuid);
}


fn main() {
    notice();
    let addr = "127.0.0.1:7878";
    println!("Dev Hash: {}", hashed_secret());
    println!("Listening for requests at http://{}", addr);
    gotham::start(addr, http_server::router())
}



