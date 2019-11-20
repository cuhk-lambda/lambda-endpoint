use crate::config::global_config;
use diesel::{PgConnection, Connection, ConnectionResult};

pub fn build_connection() -> ConnectionResult<PgConnection> {
    let database = &global_config().database_config;
    let url = format!("postgres://{}:{}@{}:{}/{}", database.username,
                      database.password, database.address, database.port, database.database);
    PgConnection::establish(url.as_str())
}