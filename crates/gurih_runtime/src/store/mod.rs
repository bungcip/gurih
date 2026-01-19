use async_trait::async_trait;
use serde_json::Value;
use sqlx::{PgPool, SqlitePool};
use std::sync::Arc;

pub mod postgres;
pub mod sqlite;

#[async_trait]
pub trait Storage: Send + Sync {
    async fn insert(&self, entity: &str, record: Value) -> Result<String, String>;
    async fn get(&self, entity: &str, id: &str) -> Result<Option<Arc<Value>>, String>;
    async fn update(&self, entity: &str, id: &str, record: Value) -> Result<(), String>;
    async fn delete(&self, entity: &str, id: &str) -> Result<(), String>;
    async fn list(&self, entity: &str, limit: Option<usize>, offset: Option<usize>) -> Result<Vec<Arc<Value>>, String>;
}

#[derive(Clone)]
pub enum DbPool {
    Sqlite(SqlitePool),
    Postgres(PgPool),
}
