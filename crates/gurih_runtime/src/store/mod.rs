use async_trait::async_trait;
use serde_json::Value;
use sqlx::{PgPool, SqlitePool};
use std::collections::HashMap;
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
    async fn find(&self, entity: &str, filters: HashMap<String, String>) -> Result<Vec<Arc<Value>>, String>;
    async fn count(&self, entity: &str, filters: HashMap<String, String>) -> Result<i64, String>;
    async fn aggregate(
        &self,
        entity: &str,
        group_by: &str,
        filters: HashMap<String, String>,
    ) -> Result<Vec<(String, i64)>, String>;
}

#[derive(Clone)]
pub enum DbPool {
    Sqlite(SqlitePool),
    Postgres(PgPool),
}
