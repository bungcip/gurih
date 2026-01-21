use sqlx::{PgPool, SqlitePool};

pub mod postgres;
pub mod sqlite;

pub use crate::datastore::DataStore;

#[derive(Clone)]
pub enum DbPool {
    Sqlite(SqlitePool),
    Postgres(PgPool),
}
