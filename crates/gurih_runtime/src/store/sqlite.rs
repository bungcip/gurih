use super::Storage;
use async_trait::async_trait;
use chrono::NaiveDate;
use serde_json::Value;
use sqlx::{Column, Row, SqlitePool, TypeInfo};
use std::sync::Arc;

pub struct SqliteStorage {
    pool: SqlitePool,
}

impl SqliteStorage {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    fn row_to_json(row: &sqlx::sqlite::SqliteRow) -> Value {
        let mut map = serde_json::Map::new();
        for col in row.columns() {
            let name = col.name();
            let type_name = col.type_info().name();

            let val = match type_name {
                "TEXT" | "VARCHAR" | "CHAR" | "CLOB" => {
                    let v: Option<String> = row.try_get(name).unwrap_or(None);
                    serde_json::to_value(v).unwrap()
                }
                "INTEGER" | "INT" | "BIGINT" | "TINYINT" | "SMALLINT" => {
                    let v: Option<i64> = row.try_get(name).unwrap_or(None);
                    serde_json::to_value(v).unwrap()
                }
                "REAL" | "FLOAT" | "DOUBLE" => {
                    let v: Option<f64> = row.try_get(name).unwrap_or(None);
                    serde_json::to_value(v).unwrap()
                }
                "BOOLEAN" => {
                    let v: Option<bool> = row.try_get(name).unwrap_or(None);
                    serde_json::to_value(v).unwrap()
                }
                "DATE" => {
                    // SQLite doesn't have a native DATE type, but we might have declared it as DATE.
                    // SQLx maps DATE to chrono::NaiveDate if the column type is "DATE".
                    let v: Option<NaiveDate> = row.try_get(name).unwrap_or(None);
                    serde_json::to_value(v).unwrap()
                }
                "DATETIME" | "TIMESTAMP" => {
                    let v: Option<chrono::NaiveDateTime> = row.try_get(name).unwrap_or(None);
                    serde_json::to_value(v).unwrap()
                }
                _ => {
                    // Fallback to string
                    let v: Option<String> = row.try_get(name).ok();
                    serde_json::to_value(v).unwrap_or(Value::Null)
                }
            };
            map.insert(name.to_string(), val);
        }
        Value::Object(map)
    }
}

#[async_trait]
impl Storage for SqliteStorage {
    async fn insert(&self, entity: &str, record: Value) -> Result<String, String> {
        let obj = record.as_object().ok_or("Record must be object")?;
        let mut query = format!("INSERT INTO \"{}\" (", entity);
        let mut params = vec![];
        let mut values_clause = String::from(") VALUES (");

        for (i, (k, v)) in obj.iter().enumerate() {
            if i > 0 {
                query.push_str(", ");
                values_clause.push_str(", ");
            }
            query.push_str(&format!("\"{}\"", k));
            values_clause.push_str(&format!("?")); // SQLite uses ?
            params.push(v);
        }

        query.push_str(&values_clause);
        query.push_str(") RETURNING id");

        let mut q = sqlx::query(&query);
        for p in params {
            match p {
                Value::String(s) => q = q.bind(s),
                Value::Number(n) => {
                    if n.is_i64() {
                        q = q.bind(n.as_i64())
                    } else if n.is_f64() {
                        q = q.bind(n.as_f64())
                    } else {
                        q = q.bind(n.to_string())
                    }
                }
                Value::Bool(b) => q = q.bind(b),
                Value::Null => q = q.bind(Option::<String>::None),
                _ => q = q.bind(p.to_string()),
            }
        }

        let row = q.fetch_one(&self.pool).await.map_err(|e| e.to_string())?;
        let id: String = row
            .try_get("id")
            .unwrap_or_else(|_| row.try_get::<i64, _>("id").map(|i| i.to_string()).unwrap_or_default());
        Ok(id)
    }

    async fn get(&self, entity: &str, id: &str) -> Result<Option<Arc<Value>>, String> {
        let query = format!("SELECT * FROM \"{}\" WHERE id = ?", entity);
        let row = sqlx::query(&query)
            .bind(id)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| e.to_string())?;

        if let Some(row) = row {
            Ok(Some(Arc::new(Self::row_to_json(&row))))
        } else {
            Ok(None)
        }
    }

    async fn update(&self, entity: &str, id: &str, record: Value) -> Result<(), String> {
        let obj = record.as_object().ok_or("Record must be object")?;
        let mut query = format!("UPDATE \"{}\" SET ", entity);
        let mut params = vec![];

        let mut i = 0;
        for (k, v) in obj {
            if k == "id" {
                continue;
            }
            if i > 0 {
                query.push_str(", ");
            }
            query.push_str(&format!("\"{}\" = ?", k));
            params.push(v);
            i += 1;
        }

        query.push_str(" WHERE id = ?");
        // params.push(&Value::String(id.to_string())); // Removed to fix temporary value error

        let mut q = sqlx::query(&query);
        for p in params {
            match p {
                Value::String(s) => q = q.bind(s),
                Value::Number(n) => {
                    if n.is_i64() {
                        q = q.bind(n.as_i64())
                    } else if n.is_f64() {
                        q = q.bind(n.as_f64())
                    } else {
                        q = q.bind(n.to_string())
                    }
                }
                Value::Bool(b) => q = q.bind(b),
                Value::Null => q = q.bind(Option::<String>::None),
                _ => q = q.bind(p.to_string()),
            }
        }
        q = q.bind(id);

        q.execute(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(())
    }

    async fn delete(&self, entity: &str, id: &str) -> Result<(), String> {
        let query = format!("DELETE FROM \"{}\" WHERE id = ?", entity);
        sqlx::query(&query)
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    async fn list(&self, entity: &str, limit: Option<usize>, offset: Option<usize>) -> Result<Vec<Arc<Value>>, String> {
        let mut query = format!("SELECT * FROM \"{}\"", entity);

        if let Some(l) = limit {
            query.push_str(&format!(" LIMIT {}", l));
        }
        if let Some(o) = offset {
            query.push_str(&format!(" OFFSET {}", o));
        }

        let rows = sqlx::query(&query)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| e.to_string())?;
        Ok(rows.iter().map(|r| Arc::new(Self::row_to_json(r))).collect())
    }
}
