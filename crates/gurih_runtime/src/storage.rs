use async_trait::async_trait;
use serde_json::Value;
// use sqlx::any::AnyRow; // Removed
// use sqlx::{AnyPool, Column, Row, TypeInfo}; // Removed
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use uuid::Uuid;

#[async_trait]
pub trait Storage: Send + Sync {
    async fn insert(&self, entity: &str, record: Value) -> Result<String, String>;
    async fn get(&self, entity: &str, id: &str) -> Result<Option<Arc<Value>>, String>;
    async fn update(&self, entity: &str, id: &str, record: Value) -> Result<(), String>;
    async fn delete(&self, entity: &str, id: &str) -> Result<(), String>;
    async fn list(&self, entity: &str, limit: Option<usize>, offset: Option<usize>) -> Result<Vec<Arc<Value>>, String>;
}

pub struct MemoryStorage {
    data: Arc<Mutex<HashMap<String, HashMap<String, Arc<Value>>>>>,
}

impl Default for MemoryStorage {
    fn default() -> Self {
        Self::new()
    }
}

impl MemoryStorage {
    pub fn new() -> Self {
        Self {
            data: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}

#[async_trait]
impl Storage for MemoryStorage {
    async fn insert(&self, entity: &str, mut record: Value) -> Result<String, String> {
        let mut data = self.data.lock().unwrap();
        let table = data.entry(entity.to_string()).or_default();

        let id = Uuid::new_v4().to_string();
        if let Some(obj) = record.as_object_mut() {
            obj.insert("id".to_string(), Value::String(id.clone()));
        }

        table.insert(id.clone(), Arc::new(record));
        Ok(id)
    }

    async fn get(&self, entity: &str, id: &str) -> Result<Option<Arc<Value>>, String> {
        let data = self.data.lock().unwrap();
        if let Some(table) = data.get(entity) {
            Ok(table.get(id).cloned())
        } else {
            Ok(None)
        }
    }

    async fn update(&self, entity: &str, id: &str, record: Value) -> Result<(), String> {
        let mut data = self.data.lock().unwrap();
        let table = data.entry(entity.to_string()).or_default();

        if table.contains_key(id) {
            let mut record = record;
            if let Some(obj) = record.as_object_mut() {
                obj.insert("id".to_string(), Value::String(id.to_string()));
            }
            table.insert(id.to_string(), Arc::new(record));
            Ok(())
        } else {
            Err("Record not found".to_string())
        }
    }

    async fn delete(&self, entity: &str, id: &str) -> Result<(), String> {
        let mut data = self.data.lock().unwrap();
        if let Some(table) = data.get_mut(entity) {
            if table.remove(id).is_some() {
                Ok(())
            } else {
                Err("Record not found".to_string())
            }
        } else {
            Err("Record not found".to_string())
        }
    }

    async fn list(&self, entity: &str, limit: Option<usize>, offset: Option<usize>) -> Result<Vec<Arc<Value>>, String> {
        let data = self.data.lock().unwrap();
        if let Some(table) = data.get(entity) {
            let skip = offset.unwrap_or(0);
            let take = limit.unwrap_or(usize::MAX);
            Ok(table.values().skip(skip).take(take).cloned().collect())
        } else {
            Ok(vec![])
        }
    }
}

use crate::store::postgres::PostgresStorage;
use crate::store::sqlite::SqliteStorage;
use crate::store::{DbPool, Storage as BackendStorage};

pub struct DatabaseStorage {
    pool: DbPool,
    sqlite: SqliteStorage,
    postgres: PostgresStorage,
}

impl DatabaseStorage {
    pub fn new(pool: DbPool) -> Self {
        match &pool {
            DbPool::Sqlite(p) => Self {
                pool: pool.clone(),
                sqlite: SqliteStorage::new(p.clone()),
                postgres: PostgresStorage::new(sqlx::PgPool::connect_lazy("postgres://").unwrap()), // Dummy
            },
            DbPool::Postgres(p) => Self {
                pool: pool.clone(),
                sqlite: SqliteStorage::new(sqlx::SqlitePool::connect_lazy("sqlite::memory:").unwrap()), // Dummy
                postgres: PostgresStorage::new(p.clone()),
            },
        }
    }
}

#[async_trait]
impl Storage for DatabaseStorage {
    async fn insert(&self, entity: &str, record: Value) -> Result<String, String> {
        match &self.pool {
            DbPool::Sqlite(_) => self.sqlite.insert(entity, record).await,
            DbPool::Postgres(_) => self.postgres.insert(entity, record).await,
        }
    }

    async fn get(&self, entity: &str, id: &str) -> Result<Option<Arc<Value>>, String> {
        match &self.pool {
            DbPool::Sqlite(_) => self.sqlite.get(entity, id).await,
            DbPool::Postgres(_) => self.postgres.get(entity, id).await,
        }
    }

    async fn update(&self, entity: &str, id: &str, record: Value) -> Result<(), String> {
        match &self.pool {
            DbPool::Sqlite(_) => self.sqlite.update(entity, id, record).await,
            DbPool::Postgres(_) => self.postgres.update(entity, id, record).await,
        }
    }

    async fn delete(&self, entity: &str, id: &str) -> Result<(), String> {
        match &self.pool {
            DbPool::Sqlite(_) => self.sqlite.delete(entity, id).await,
            DbPool::Postgres(_) => self.postgres.delete(entity, id).await,
        }
    }

    async fn list(&self, entity: &str, limit: Option<usize>, offset: Option<usize>) -> Result<Vec<Arc<Value>>, String> {
        match &self.pool {
            DbPool::Sqlite(_) => self.sqlite.list(entity, limit, offset).await,
            DbPool::Postgres(_) => self.postgres.list(entity, limit, offset).await,
        }
    }
}
