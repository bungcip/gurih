use super::{DataStore, validate_identifier};
use async_trait::async_trait;
use chrono::NaiveDate;
use serde_json::Value;
use sqlx::{Column, Row, SqlitePool, TypeInfo};
use std::collections::HashMap;
use std::sync::Arc;

pub struct SqliteDataStore {
    pool: SqlitePool,
}

impl SqliteDataStore {
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
impl DataStore for SqliteDataStore {
    async fn insert(&self, entity: &str, record: Value) -> Result<String, String> {
        validate_identifier(entity)?;
        let obj = record.as_object().ok_or("Record must be object")?;

        // Estimate capacity to reduce reallocations
        // A rough guess: each field adds ~10-20 chars for column name + overhead, and 2-3 chars for value placeholder
        let estimated_cols = obj.len() * 16;
        let estimated_vals = obj.len() * 3;

        let mut query = String::with_capacity(30 + entity.len() + estimated_cols + estimated_vals);
        query.push_str("INSERT INTO \"");
        query.push_str(entity);
        query.push_str("\" (");

        let mut values_clause = String::with_capacity(12 + estimated_vals);
        values_clause.push_str(") VALUES (");

        let mut params = Vec::with_capacity(obj.len());

        for (i, (k, v)) in obj.iter().enumerate() {
            validate_identifier(k)?;
            if i > 0 {
                query.push_str(", ");
                values_clause.push_str(", ");
            }
            query.push('"');
            query.push_str(k);
            query.push('"');
            values_clause.push('?'); // SQLite uses ?
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
        validate_identifier(entity)?;
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
        validate_identifier(entity)?;
        let obj = record.as_object().ok_or("Record must be object")?;
        let mut query = format!("UPDATE \"{}\" SET ", entity);
        let mut params = vec![];

        let mut i = 0;
        for (k, v) in obj {
            if k == "id" {
                continue;
            }
            validate_identifier(k)?;
            if i > 0 {
                query.push_str(", ");
            }
            query.push_str(&format!("\"{}\" = ?", k));
            params.push(v);
            i += 1;
        }

        query.push_str(" WHERE id = ?");

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
        validate_identifier(entity)?;
        let query = format!("DELETE FROM \"{}\" WHERE id = ?", entity);
        sqlx::query(&query)
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    async fn list(&self, entity: &str, limit: Option<usize>, offset: Option<usize>) -> Result<Vec<Arc<Value>>, String> {
        validate_identifier(entity)?;
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

    async fn find(&self, entity: &str, filters: HashMap<String, String>) -> Result<Vec<Arc<Value>>, String> {
        validate_identifier(entity)?;
        let mut query = format!("SELECT * FROM \"{}\"", entity);
        let mut params = vec![];

        if !filters.is_empty() {
            query.push_str(" WHERE ");
            for (i, (k, v)) in filters.iter().enumerate() {
                validate_identifier(k)?;
                if i > 0 {
                    query.push_str(" AND ");
                }
                query.push_str(&format!("\"{}\" = ?", k));
                params.push(v);
            }
        }

        let mut q = sqlx::query(&query);
        for p in params {
            q = q.bind(p);
        }

        let rows = q.fetch_all(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(rows.iter().map(|r| Arc::new(Self::row_to_json(r))).collect())
    }

    async fn count(&self, entity: &str, filters: HashMap<String, String>) -> Result<i64, String> {
        validate_identifier(entity)?;
        let mut query = format!("SELECT COUNT(*) FROM \"{}\"", entity);
        let mut params = vec![];

        if !filters.is_empty() {
            query.push_str(" WHERE ");
            for (i, (k, v)) in filters.iter().enumerate() {
                validate_identifier(k)?;
                if i > 0 {
                    query.push_str(" AND ");
                }
                query.push_str(&format!("\"{}\" = ?", k));
                params.push(v);
            }
        }

        let mut q = sqlx::query_scalar(&query);
        for p in params {
            q = q.bind(p);
        }

        let count: i64 = q.fetch_one(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(count)
    }

    async fn aggregate(
        &self,
        entity: &str,
        group_by: &str,
        filters: HashMap<String, String>,
    ) -> Result<Vec<(String, i64)>, String> {
        validate_identifier(entity)?;
        validate_identifier(group_by)?;
        let mut query = format!("SELECT \"{}\", COUNT(*) FROM \"{}\"", group_by, entity);
        let mut params = vec![];

        if !filters.is_empty() {
            query.push_str(" WHERE ");
            for (i, (k, v)) in filters.iter().enumerate() {
                validate_identifier(k)?;
                if i > 0 {
                    query.push_str(" AND ");
                }
                query.push_str(&format!("\"{}\" = ?", k));
                params.push(v);
            }
        }

        query.push_str(&format!(" GROUP BY \"{}\"", group_by));

        let mut q = sqlx::query(&query);
        for p in params {
            q = q.bind(p);
        }

        let rows = q.fetch_all(&self.pool).await.map_err(|e| e.to_string())?;

        let mut results = vec![];
        for row in rows {
            let key: String = row
                .try_get(0)
                .unwrap_or_else(|_| row.try_get::<i64, _>(0).map(|i| i.to_string()).unwrap_or_default());
            let count: i64 = row.try_get(1).unwrap_or(0);
            results.push((key, count));
        }

        Ok(results)
    }

    async fn query(&self, sql: &str) -> Result<Vec<Arc<Value>>, String> {
        let rows = sqlx::query(sql)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| e.to_string())?;
        Ok(rows.iter().map(|r| Arc::new(Self::row_to_json(r))).collect())
    }
}
