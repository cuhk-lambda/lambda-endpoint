#[global_allocator]
static ALLOC: jemallocator::Jemalloc = jemallocator::Jemalloc;
mod config;
mod cli;
mod db;
#[macro_use]
extern crate lazy_static;
extern crate serde;
#[macro_use]
extern crate diesel;
use db::schema::trace::traces::dsl::*;
use db::model::trace::*;
use self::diesel::prelude::*;
fn main() {
    let c = config::global_config();
    println!("{:?}", c);
    let conn = db::connection::build_connection().unwrap();
    let a = traces.load::<Trace>(&conn).unwrap();
}
