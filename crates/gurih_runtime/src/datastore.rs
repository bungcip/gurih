use async_trait::async_trait;
use serde_json::Value;
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
    async fn query_with_params(&self, sql: &str, params: Vec<Value>) -> Result<Vec<Arc<Value>>, String>;
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

struct CompiledFilter {
    key: String,
    val: String,
    val_i64: Option<i64>,
    val_f64: Option<f64>,
    val_bool: Option<bool>,
}

impl CompiledFilter {
    fn new(key: String, val: String) -> Self {
        Self {
            val_i64: val.parse::<i64>().ok(),
            val_f64: val.parse::<f64>().ok(),
            val_bool: val.parse::<bool>().ok(),
            key,
            val,
        }
    }

    fn matches(&self, record: &Value) -> bool {
        match record.get(&self.key) {
            Some(Value::String(s)) => s == &self.val,
            Some(Value::Number(n)) => {
                if let (Some(i), Some(vi)) = (n.as_i64(), self.val_i64) {
                    i == vi
                } else if let (Some(f), Some(vf)) = (n.as_f64(), self.val_f64) {
                    (f - vf).abs() < f64::EPSILON
                } else if self.val_i64.is_none() && self.val_f64.is_none() && (n.is_i64() || n.is_f64()) {
                    false
                } else {
                    n.to_string() == self.val
                }
            }
            Some(Value::Bool(b)) => {
                if let Some(vb) = self.val_bool {
                    *b == vb
                } else {
                    false
                }
            }
            _ => false,
        }
    }
}

impl MemoryDataStore {
    pub fn new() -> Self {
        Self {
            data: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    fn matches_filters(record: &Value, filters: &[CompiledFilter]) -> bool {
        for filter in filters {
            if !filter.matches(record) {
                return false;
            }
        }
        true
    }
}

#[async_trait]
impl DataStore for MemoryDataStore {
    async fn insert(&self, entity: &str, mut record: Value) -> Result<String, String> {
        let mut data = self.data.lock().unwrap();
        let table = data.entry(entity.to_string()).or_default();

        let id = if let Some(existing_id) = record.get("id").and_then(|v| v.as_str()) {
            existing_id.to_string()
        } else {
            Uuid::new_v4().to_string()
        };

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
                if let Value::Object(source) = record {
                    for (k, v) in source {
                        target.insert(k, v);
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
            // OPTIMIZATION: Pre-process filters to avoid parsing/allocating inside the loop
            let parsed_filters: Vec<CompiledFilter> = filters
                .into_iter()
                .map(|(k, v)| CompiledFilter::new(k, v))
                .collect();

            let results: Vec<Arc<Value>> = table
                .values()
                .filter(|record| Self::matches_filters(record, &parsed_filters))
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
            // OPTIMIZATION: Pre-process filters to avoid parsing/allocating inside the loop
            let parsed_filters: Vec<CompiledFilter> = filters
                .into_iter()
                .map(|(k, v)| CompiledFilter::new(k, v))
                .collect();

            let count = table
                .values()
                .filter(|record| Self::matches_filters(record, &parsed_filters))
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

            // OPTIMIZATION: Pre-process filters to avoid parsing/allocating inside the loop
            let parsed_filters: Vec<CompiledFilter> = filters
                .into_iter()
                .map(|(k, v)| CompiledFilter::new(k, v))
                .collect();

            for record in table.values() {
                // Filter first
                if !Self::matches_filters(record, &parsed_filters) {
                    continue;
                }

                // Group
                let group_key_ref = record
                    .get(group_by)
                    .and_then(|v| v.as_str())
                    .unwrap_or("Unknown");

                if let Some(count) = groups.get_mut(group_key_ref) {
                    *count += 1;
                } else {
                    groups.insert(group_key_ref.to_string(), 1);
                }
            }

            Ok(groups.into_iter().collect())
        } else {
            Ok(vec![])
        }
    }

    async fn query(&self, _sql: &str) -> Result<Vec<Arc<Value>>, String> {
        Err("Raw SQL query not supported in MemoryDataStore".to_string())
    }

    async fn query_with_params(&self, _sql: &str, _params: Vec<Value>) -> Result<Vec<Arc<Value>>, String> {
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

pub async fn init_datastore(schema: Arc<Schema>, base_path: Option<&Path>) -> Result<Arc<dyn DataStore>, String> {
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

        if db_config.db_type == DatabaseType::Sqlite {
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
                        std::env::current_dir().map_err(|e| e.to_string())?.join(path_obj)
                    }
                } else {
                    path_obj.to_path_buf()
                };

                // Ensure absolute
                if full_path.is_relative() {
                    full_path = std::env::current_dir().map_err(|e| e.to_string())?.join(full_path);
                }

                // Ensure parent directory exists
                if let Some(parent) = full_path.parent()
                    && !parent.as_os_str().is_empty()
                    && !parent.exists()
                {
                    std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
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

            let pool = DbPool::Sqlite(p.clone());
            let manager = SchemaManager::new(pool, schema.clone(), db_config.db_type.clone());
            manager.migrate().await?;

            Ok(Arc::new(SqliteDataStore::new(p)))
        } else if db_config.db_type == DatabaseType::Postgres {
            let p = PgPoolOptions::new()
                .max_connections(5)
                .connect(&url)
                .await
                .map_err(|e| format!("Failed to connect to Postgres DB: {}", e))?;

            let pool = DbPool::Postgres(p.clone());
            let manager = SchemaManager::new(pool, schema.clone(), db_config.db_type.clone());
            manager.migrate().await?;

            Ok(Arc::new(PostgresDataStore::new(p)))
        } else {
            Err(format!("Unsupported database type: {:?}", db_config.db_type))
        }
    } else {
        println!("⚠️ No database configured. Using in-memory datastore.");
        Ok(Arc::new(MemoryDataStore::new()))
    }
}
