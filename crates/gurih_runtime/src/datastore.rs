use async_trait::async_trait;
use serde_json::Value;
// use sqlx::any::AnyRow; // Removed
// use sqlx::{AnyPool, Column, Row, TypeInfo}; // Removed
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use uuid::Uuid;

#[async_trait]
pub trait DataStore: Send + Sync {
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
    async fn query(&self, sql: &str) -> Result<Vec<Arc<Value>>, String>;
}

type DataStoreData = HashMap<String, HashMap<String, Arc<Value>>>;

pub struct MemoryDataStore {
    data: Arc<Mutex<DataStoreData>>,
}

impl Default for MemoryDataStore {
    fn default() -> Self {
        Self::new()
    }
}

impl MemoryDataStore {
    pub fn new() -> Self {
        Self {
            data: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}

#[async_trait]
impl DataStore for MemoryDataStore {
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

        if let Some(existing) = table.get(id) {
            // Merge existing with new record
            let mut new_record = (**existing).clone();

            if let Some(target) = new_record.as_object_mut() {
                if let Some(source) = record.as_object() {
                    for (k, v) in source {
                        target.insert(k.clone(), v.clone());
                    }
                }
            }

            // Ensure ID is preserved/set
            if let Some(obj) = new_record.as_object_mut() {
                obj.insert("id".to_string(), Value::String(id.to_string()));
            }

            table.insert(id.to_string(), Arc::new(new_record));
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

    async fn find(&self, entity: &str, filters: HashMap<String, String>) -> Result<Vec<Arc<Value>>, String> {
        let data = self.data.lock().unwrap();
        if let Some(table) = data.get(entity) {
            let results: Vec<Arc<Value>> = table
                .values()
                .filter(|record| {
                    for (k, v) in &filters {
                        if let Some(val) = record.get(k).and_then(|val| val.as_str()) {
                            if val != v {
                                return false;
                            }
                        } else {
                            return false;
                        }
                    }
                    true
                })
                .cloned()
                .collect();
            Ok(results)
        } else {
            Ok(vec![])
        }
    }

    async fn count(&self, entity: &str, filters: HashMap<String, String>) -> Result<i64, String> {
        let data = self.data.lock().unwrap();
        if let Some(table) = data.get(entity) {
            let count = table
                .values()
                .filter(|record| {
                    for (k, v) in &filters {
                        if let Some(val) = record.get(k).and_then(|val| val.as_str()) {
                            if val != v {
                                return false;
                            }
                        } else {
                            // If field is missing or not a string, for now assume no match
                            // Ideally handle other types by converting to string
                            return false;
                        }
                    }
                    true
                })
                .count();
            Ok(count as i64)
        } else {
            Ok(0)
        }
    }

    async fn aggregate(
        &self,
        entity: &str,
        group_by: &str,
        filters: HashMap<String, String>,
    ) -> Result<Vec<(String, i64)>, String> {
        let data = self.data.lock().unwrap();
        if let Some(table) = data.get(entity) {
            let mut groups: HashMap<String, i64> = HashMap::new();

            for record in table.values() {
                // Filter first
                let mut match_filter = true;
                for (k, v) in &filters {
                    if let Some(val) = record.get(k).and_then(|val| val.as_str()) {
                        if val != v {
                            match_filter = false;
                            break;
                        }
                    } else {
                        match_filter = false;
                        break;
                    }
                }
                if !match_filter {
                    continue;
                }

                // Group
                let group_key = record
                    .get(group_by)
                    .and_then(|v| v.as_str())
                    .unwrap_or("Unknown")
                    .to_string();
                *groups.entry(group_key).or_insert(0) += 1;
            }

            Ok(groups.into_iter().collect())
        } else {
            Ok(vec![])
        }
    }

    async fn query(&self, _sql: &str) -> Result<Vec<Arc<Value>>, String> {
        Err("Raw SQL query not supported in MemoryDataStore".to_string())
    }
}

use crate::persistence::SchemaManager;
use crate::store::DbPool;
use crate::store::postgres::PostgresDataStore;
use crate::store::sqlite::SqliteDataStore;
use gurih_ir::{DatabaseType, Schema};
use sqlx::postgres::PgPoolOptions;
use sqlx::sqlite::SqlitePoolOptions;
use std::path::Path;

pub async fn init_datastore(
    schema: Arc<Schema>,
    base_path: Option<&Path>,
) -> Result<Arc<dyn DataStore>, String> {
    if let Some(db_config) = &schema.database {
        sqlx::any::install_default_drivers();
        println!("🔌 Connecting to database...");
        // Handle env:DATABASE_URL
        let url = if db_config.url.starts_with("env:") {
            std::env::var(&db_config.url[4..]).unwrap_or_else(|_| "".to_string())
        } else {
            db_config.url.clone()
        };

        if url.is_empty() {
            return Err("Database URL is empty or env var not set.".to_string());
        }

        let pool = if db_config.db_type == DatabaseType::Sqlite {
            let mut db_path = url
                .trim_start_matches("sqlite://")
                .trim_start_matches("sqlite:")
                .trim_start_matches("file:")
                .to_string();

            if db_path != ":memory:" {
                let path_obj = Path::new(&db_path);
                let mut full_path = if path_obj.is_relative() {
                    if let Some(parent) = base_path {
                        parent.join(path_obj)
                    } else {
                        std::env::current_dir()
                            .map_err(|e| e.to_string())?
                            .join(path_obj)
                    }
                } else {
                    path_obj.to_path_buf()
                };

                // Ensure absolute
                if full_path.is_relative() {
                    full_path = std::env::current_dir()
                        .map_err(|e| e.to_string())?
                        .join(full_path);
                }

                // Ensure parent directory exists
                if let Some(parent) = full_path.parent() {
                    if !parent.as_os_str().is_empty() && !parent.exists() {
                        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
                    }
                }

                // Explicitly create file if not exists
                if !full_path.exists() {
                    std::fs::File::create(&full_path).map_err(|e| e.to_string())?;
                }

                db_path = full_path.to_string_lossy().to_string();
            }

            let url = if db_path == ":memory:" {
                "sqlite::memory:".to_string()
            } else {
                format!("sqlite://{}", db_path)
            };

            let p = SqlitePoolOptions::new()
                .max_connections(5)
                .connect(&url)
                .await
                .map_err(|e| format!("Failed to connect to SQLite DB at {}: {}", url, e))?;
            DbPool::Sqlite(p)
        } else if db_config.db_type == DatabaseType::Postgres {
            let p = PgPoolOptions::new()
                .max_connections(5)
                .connect(&url)
                .await
                .map_err(|e| format!("Failed to connect to Postgres DB: {}", e))?;
            DbPool::Postgres(p)
        } else {
            return Err(format!("Unsupported database type: {:?}", db_config.db_type));
        };

        let manager =
            SchemaManager::new(pool.clone(), schema.clone(), format!("{:?}", db_config.db_type));
        manager.migrate().await?;

        Ok(Arc::new(DatabaseDataStore::new(pool)))
    } else {
        println!("⚠️ No database configured. Using in-memory datastore.");
        Ok(Arc::new(MemoryDataStore::new()))
    }
}

pub struct DatabaseDataStore {
    pool: DbPool,
    sqlite: SqliteDataStore,
    postgres: PostgresDataStore,
}

impl DatabaseDataStore {
    pub fn new(pool: DbPool) -> Self {
        match &pool {
            DbPool::Sqlite(p) => Self {
                pool: pool.clone(),
                sqlite: SqliteDataStore::new(p.clone()),
                postgres: PostgresDataStore::new(sqlx::PgPool::connect_lazy("postgres://").unwrap()), // Dummy
            },
            DbPool::Postgres(p) => Self {
                pool: pool.clone(),
                sqlite: SqliteDataStore::new(sqlx::SqlitePool::connect_lazy("sqlite::memory:").unwrap()), // Dummy
                postgres: PostgresDataStore::new(p.clone()),
            },
        }
    }
}

#[async_trait]
impl DataStore for DatabaseDataStore {
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

    async fn find(&self, entity: &str, filters: HashMap<String, String>) -> Result<Vec<Arc<Value>>, String> {
        match &self.pool {
            DbPool::Sqlite(_) => self.sqlite.find(entity, filters).await,
            DbPool::Postgres(_) => self.postgres.find(entity, filters).await,
        }
    }

    async fn count(&self, entity: &str, filters: HashMap<String, String>) -> Result<i64, String> {
        match &self.pool {
            DbPool::Sqlite(_) => self.sqlite.count(entity, filters).await,
            DbPool::Postgres(_) => self.postgres.count(entity, filters).await,
        }
    }

    async fn aggregate(
        &self,
        entity: &str,
        group_by: &str,
        filters: HashMap<String, String>,
    ) -> Result<Vec<(String, i64)>, String> {
        match &self.pool {
            DbPool::Sqlite(_) => self.sqlite.aggregate(entity, group_by, filters).await,
            DbPool::Postgres(_) => self.postgres.aggregate(entity, group_by, filters).await,
        }
    }

    async fn query(&self, sql: &str) -> Result<Vec<Arc<Value>>, String> {
        match &self.pool {
            DbPool::Sqlite(_) => self.sqlite.query(sql).await,
            DbPool::Postgres(_) => self.postgres.query(sql).await,
        }
    }
}
