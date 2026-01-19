use async_trait::async_trait;
use serde_json::Value;
use sqlx::any::AnyRow;
use sqlx::{AnyPool, Column, Row, TypeInfo};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use uuid::Uuid;

#[async_trait]
pub trait Storage: Send + Sync {
    async fn insert(&self, entity: &str, record: Value) -> Result<String, String>;
    async fn get(&self, entity: &str, id: &str) -> Result<Option<Value>, String>;
    async fn update(&self, entity: &str, id: &str, record: Value) -> Result<(), String>;
    async fn delete(&self, entity: &str, id: &str) -> Result<(), String>;
    async fn list(&self, entity: &str) -> Result<Vec<Value>, String>;
}

pub struct MemoryStorage {
    data: Arc<Mutex<HashMap<String, HashMap<String, Value>>>>,
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

        table.insert(id.clone(), record);
        Ok(id)
    }

    async fn get(&self, entity: &str, id: &str) -> Result<Option<Value>, String> {
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
            table.insert(id.to_string(), record);
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

    async fn list(&self, entity: &str) -> Result<Vec<Value>, String> {
        let data = self.data.lock().unwrap();
        if let Some(table) = data.get(entity) {
            Ok(table.values().cloned().collect())
        } else {
            Ok(vec![])
        }
    }
}

pub struct AnyStorage {
    pool: AnyPool,
}

impl AnyStorage {
    pub fn new(pool: AnyPool) -> Self {
        Self { pool }
    }

    fn row_to_json(row: AnyRow) -> Value {
        let mut map = serde_json::Map::new();
        for col in row.columns() {
            let name = col.name();
            let type_info = col.type_info();
            let type_name = type_info.name();

            match type_name {
                "TEXT" | "VARCHAR" | "CHAR" | "NAME" | "STRING" => {
                    let val: Option<String> = row.try_get(name).ok();
                    map.insert(name.to_string(), serde_json::to_value(val).unwrap());
                }
                "INT4" | "INT8" | "INTEGER" | "INT" => {
                    let val: Option<i64> = row.try_get(name).ok();
                    map.insert(name.to_string(), serde_json::to_value(val).unwrap());
                }
                "BOOL" | "BOOLEAN" => {
                    let val: Option<bool> = row.try_get(name).ok();
                    map.insert(name.to_string(), serde_json::to_value(val).unwrap());
                }
                "FLOAT" | "REAL" | "DOUBLE PRECISION" | "FLOAT8" | "FLOAT4" => {
                    let val: Option<f64> = row.try_get(name).ok();
                    map.insert(name.to_string(), serde_json::to_value(val).unwrap());
                }
                _ => {
                    // Try as string if unknown
                    let val: Option<String> = row.try_get(name).ok();
                    map.insert(
                        name.to_string(),
                        serde_json::to_value(val).unwrap_or(Value::Null),
                    );
                }
            }
        }
        Value::Object(map)
    }
}

#[async_trait]
impl Storage for AnyStorage {
    async fn insert(&self, entity: &str, record: Value) -> Result<String, String> {
        let obj = record.as_object().ok_or("Record must be object")?;

        let mut query = format!("INSERT INTO \"{}\" (", entity);
        let mut params = vec![];
        let mut values_clause = String::from(") VALUES (");

        let mut i = 1;
        for (k, v) in obj {
            if i > 1 {
                query.push_str(", ");
                values_clause.push_str(", ");
            }
            query.push_str(&format!("\"{}\"", k));
            values_clause.push_str(&format!("${}", i));
            params.push(v);
            i += 1;
        }

        query.push_str(&values_clause);
        query.push_str(") RETURNING id");

        let mut q = sqlx::query(&query);
        for p in params {
            match p {
                Value::String(s) => q = q.bind(s),
                Value::Number(n) => {
                    if n.is_i64() {
                        q = q.bind(n.as_i64());
                    } else if n.is_f64() {
                        q = q.bind(n.as_f64());
                    } else {
                        q = q.bind(n.to_string());
                    }
                }
                Value::Bool(b) => q = q.bind(b),
                Value::Null => q = q.bind(Option::<String>::None),
                _ => q = q.bind(p.to_string()),
            }
        }

        let row = q.fetch_one(&self.pool).await.map_err(|e| e.to_string())?;

        // Try getting id as String, fallback to int
        let id_val: String = row.try_get("id").unwrap_or_else(|_| {
            row.try_get::<i64, _>("id")
                .map(|i| i.to_string())
                .unwrap_or_default()
        });

        Ok(id_val)
    }

    async fn get(&self, entity: &str, id: &str) -> Result<Option<Value>, String> {
        let query = format!("SELECT * FROM \"{}\" WHERE id = $1", entity);
        let row = sqlx::query(&query)
            .bind(id)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| e.to_string())?;

        Ok(row.map(Self::row_to_json))
    }

    async fn update(&self, entity: &str, id: &str, record: Value) -> Result<(), String> {
        let obj = record.as_object().ok_or("Record must be object")?;

        let mut query = format!("UPDATE \"{}\" SET ", entity);
        let mut params = vec![];

        let mut i = 1;
        for (k, v) in obj {
            if k == "id" {
                continue;
            } // Don't update ID
            if i > 1 {
                query.push_str(", ");
            }
            query.push_str(&format!("\"{}\" = ${}", k, i));
            params.push(v);
            i += 1;
        }

        query.push_str(&format!(" WHERE id = ${}", i));
        let id_val = Value::String(id.to_string());
        params.push(&id_val);

        let mut q = sqlx::query(&query);
        for p in params {
            match p {
                Value::String(s) => q = q.bind(s),
                Value::Number(n) => {
                    if n.is_i64() {
                        q = q.bind(n.as_i64());
                    } else if n.is_f64() {
                        q = q.bind(n.as_f64());
                    } else {
                        q = q.bind(n.to_string());
                    }
                }
                Value::Bool(b) => q = q.bind(b),
                Value::Null => q = q.bind(Option::<String>::None),
                _ => q = q.bind(p.to_string()),
            }
        }

        q.execute(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(())
    }

    async fn delete(&self, entity: &str, id: &str) -> Result<(), String> {
        let query = format!("DELETE FROM \"{}\" WHERE id = $1", entity);
        sqlx::query(&query)
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    async fn list(&self, entity: &str) -> Result<Vec<Value>, String> {
        let query = format!("SELECT * FROM \"{}\"", entity);
        let rows = sqlx::query(&query)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| e.to_string())?;

        Ok(rows.into_iter().map(Self::row_to_json).collect())
    }
}
