use crate::config::global_config;
use diesel::{PgConnection, Connection, ConnectionResult};
use std::ops::{DerefMut, Deref};
use crossbeam::queue::ArrayQueue;

fn build_connection() -> ConnectionResult<PgConnection> {
    let database = &global_config().database_config;
    let url = format!("postgres://{}:{}@{}:{}/{}", database.username,
                      database.password, database.address, database.port, database.database);
    PgConnection::establish(url.as_str())
}

pub struct ConnectionPool {
    pool: ArrayQueue<PgConnection>,
}

pub struct PooledConnection<'a> {
    inner: Option<PgConnection>,
    pool: &'a ConnectionPool
}

impl Deref for PooledConnection<'_> {
    type Target = PgConnection;
    fn deref(&self) -> &Self::Target {
        self.inner.as_ref().unwrap()
    }
}

impl DerefMut for PooledConnection<'_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.inner.as_mut().unwrap()
    }
}

impl Drop for PooledConnection<'_> {
    fn drop(&mut self) {
        let inner = self.inner.take().unwrap();
        self.pool.recycle(inner);
    }
}
impl ConnectionPool {
    pub fn new(size: usize) -> Self {
        ConnectionPool {
            pool: ArrayQueue::new(size)
        }
    }
    fn recycle(&self, conn: PgConnection) {
        match self.pool.push(conn) {
            Ok(_) => (),
            Err(e) => drop(e.0)
        }
    }
    fn get(&self) -> PooledConnection {
        let conn = match self.pool.pop() {
            Ok(conn) => conn,
            Err(_) => build_connection().unwrap()
        };
        PooledConnection {
            inner: Some(conn),
            pool: self
        }
    }
}

lazy_static!{
    static ref POOL : ConnectionPool = ConnectionPool::new(7);
}

pub fn get_conn() -> PooledConnection<'static> {
    POOL.get()
}

#[test]
fn basic_db() {
    use super::schema::trace::traces::dsl::*;
    use diesel::prelude::*;
    use super::model::trace::Trace;
    let conn = build_connection().unwrap();
    traces.load::<Trace>(&conn).unwrap();
}

#[test]
fn pooled_db() {
    use super::schema::trace::traces::dsl::*;
    use diesel::prelude::*;
    use super::model::trace::Trace;
    {
        let conn = get_conn();
        traces.load::<Trace>(&*conn).unwrap();
    }
    {
        let conn = get_conn();
        traces.load::<Trace>(&*conn).unwrap();
    }
}

