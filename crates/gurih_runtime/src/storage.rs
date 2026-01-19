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

#[derive(Clone, Copy, Debug)]
enum DataType {
    Text,
    Int,
    Bool,
    Float,
    Unknown,
}

fn get_column_type(type_name: &str) -> DataType {
    match type_name {
        "TEXT" | "VARCHAR" | "CHAR" | "NAME" | "STRING" => DataType::Text,
        "INT4" | "INT8" | "INTEGER" | "INT" | "BIGINT" | "smallint" | "bigint" | "int"
        | "integer" => DataType::Int,
        "BOOL" | "BOOLEAN" | "boolean" | "bool" => DataType::Bool,
        "FLOAT" | "REAL" | "DOUBLE PRECISION" | "FLOAT8" | "FLOAT4" | "numeric" => DataType::Float,
        _ => DataType::Unknown,
    }
}

pub struct AnyStorage {
    pool: AnyPool,
}

impl AnyStorage {
    pub fn new(pool: AnyPool) -> Self {
        Self { pool }
    }

    fn row_to_json_optimized(row: &AnyRow, columns: &[(String, DataType)]) -> Value {
        let mut map = serde_json::Map::new();
        for (i, (name, dtype)) in columns.iter().enumerate() {
            match dtype {
                DataType::Text => {
                    let val: Option<String> = row.try_get(i).ok();
                    map.insert(name.clone(), serde_json::to_value(val).unwrap());
                }
                DataType::Int => {
                    let val: Option<i64> = row.try_get(i).ok();
                    match val {
                        Some(v) => {
                            map.insert(name.clone(), serde_json::to_value(v).unwrap());
                        }
                        None => {
                            // Fallback to string if int fails
                            let s_val: Option<String> = row.try_get(i).ok();
                            map.insert(
                                name.clone(),
                                serde_json::to_value(s_val).unwrap_or(Value::Null),
                            );
                        }
                    }
                }
                DataType::Bool => {
                    let val: Option<bool> = row.try_get(i).ok();
                    map.insert(name.clone(), serde_json::to_value(val).unwrap());
                }
                DataType::Float => {
                    let val: Option<f64> = row.try_get(i).ok();
                    map.insert(name.clone(), serde_json::to_value(val).unwrap());
                }
                DataType::Unknown => {
                    // Try as string first
                    if let Ok(val) = row.try_get::<String, _>(i) {
                        map.insert(name.clone(), Value::String(val));
                    } else if let Ok(val) = row.try_get::<i64, _>(i) {
                        map.insert(name.clone(), serde_json::to_value(val).unwrap());
                    } else if let Ok(val) = row.try_get::<f64, _>(i) {
                        map.insert(name.clone(), serde_json::to_value(val).unwrap());
                    } else {
                        map.insert(name.clone(), Value::Null);
                    }
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

        if let Some(row) = row {
            let columns: Vec<(String, DataType)> = row
                .columns()
                .iter()
                .map(|col| {
                    (
                        col.name().to_string(),
                        get_column_type(col.type_info().name()),
                    )
                })
                .collect();
            Ok(Some(Self::row_to_json_optimized(&row, &columns)))
        } else {
            Ok(None)
        }
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
        let mut q = sqlx::query(&query);

        // Try parsing as integer first to handle numeric IDs (e.g. SQLite/Postgres strictness)
        if let Ok(int_id) = id.parse::<i64>() {
            q = q.bind(int_id);
        } else {
            q = q.bind(id);
        }

        q.execute(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(())
    }

    async fn list(&self, entity: &str) -> Result<Vec<Value>, String> {
        let query = format!("SELECT * FROM \"{}\"", entity);
        let rows = sqlx::query(&query)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| e.to_string())?;

        if rows.is_empty() {
            return Ok(vec![]);
        }

        let columns: Vec<(String, DataType)> = rows[0]
            .columns()
            .iter()
            .map(|col| {
                (
                    col.name().to_string(),
                    get_column_type(col.type_info().name()),
                )
            })
            .collect();

        Ok(rows
            .iter()
            .map(|row| Self::row_to_json_optimized(row, &columns))
            .collect())
    }
}
