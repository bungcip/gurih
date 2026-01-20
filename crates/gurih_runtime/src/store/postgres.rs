use super::Storage;
use async_trait::async_trait;
use chrono::NaiveDate;
use serde_json::Value;
use sqlx::{Column, PgPool, Row, TypeInfo};
use std::collections::HashMap;
use std::sync::Arc;

pub struct PostgresStorage {
    pool: PgPool,
}

impl PostgresStorage {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    fn row_to_json(row: &sqlx::postgres::PgRow) -> Value {
        let mut map = serde_json::Map::new();
        for col in row.columns() {
            let name = col.name();
            let type_name = col.type_info().name();

            let val = match type_name {
                "TEXT" | "VARCHAR" | "CHAR" | "NAME" => {
                    let v: Option<String> = row.try_get(name).unwrap_or(None);
                    serde_json::to_value(v).unwrap()
                }
                "INT4" | "INT8" | "INTEGER" | "BIGINT" => {
                    let v: Option<i64> = row.try_get(name).unwrap_or(None);
                    serde_json::to_value(v).unwrap()
                }
                "FLOAT4" | "FLOAT8" | "REAL" | "DOUBLE PRECISION" => {
                    let v: Option<f64> = row.try_get(name).unwrap_or(None);
                    serde_json::to_value(v).unwrap()
                }
                "BOOL" | "BOOLEAN" => {
                    let v: Option<bool> = row.try_get(name).unwrap_or(None);
                    serde_json::to_value(v).unwrap()
                }
                "DATE" => {
                    let v: Option<NaiveDate> = row.try_get(name).unwrap_or(None);
                    serde_json::to_value(v).unwrap()
                }
                "TIMESTAMP" | "TIMESTAMPTZ" => {
                    let v: Option<chrono::NaiveDateTime> = row.try_get(name).unwrap_or(None);
                    serde_json::to_value(v).unwrap()
                }
                _ => {
                    // Fallback
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
impl Storage for PostgresStorage {
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
            values_clause.push_str(&format!("${}", i + 1));
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
        // Postgres IDs might be INT or UUID/TEXT, try both
        let id: String = row
            .try_get("id")
            .unwrap_or_else(|_| row.try_get::<i64, _>("id").map(|i| i.to_string()).unwrap_or_default());
        Ok(id)
    }

    async fn get(&self, entity: &str, id: &str) -> Result<Option<Arc<Value>>, String> {
        let query = format!("SELECT * FROM \"{}\" WHERE id = $1", entity);
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
            query.push_str(&format!("\"{}\" = ${}", k, i + 1));
            params.push(v);
            i += 1;
        }

        query.push_str(&format!(" WHERE id = ${}", i + 1));
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
        let query = format!("DELETE FROM \"{}\" WHERE id = $1", entity);
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

    async fn find(&self, entity: &str, filters: HashMap<String, String>) -> Result<Vec<Arc<Value>>, String> {
        let mut query = format!("SELECT * FROM \"{}\"", entity);
        let mut params = vec![];

        if !filters.is_empty() {
            query.push_str(" WHERE ");
            for (i, (k, v)) in filters.iter().enumerate() {
                if i > 0 {
                    query.push_str(" AND ");
                }
                query.push_str(&format!("\"{}\" = ${}", k, i + 1));
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
        let mut query = format!("SELECT COUNT(*) FROM \"{}\"", entity);
        let mut params = vec![];

        if !filters.is_empty() {
            query.push_str(" WHERE ");
            for (i, (k, v)) in filters.iter().enumerate() {
                if i > 0 {
                    query.push_str(" AND ");
                }
                query.push_str(&format!("\"{}\" = ${}", k, i + 1));
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
        let mut query = format!("SELECT \"{}\", COUNT(*) FROM \"{}\"", group_by, entity);
        let mut params = vec![];

        if !filters.is_empty() {
            query.push_str(" WHERE ");
            for (i, (k, v)) in filters.iter().enumerate() {
                if i > 0 {
                    query.push_str(" AND ");
                }
                query.push_str(&format!("\"{}\" = ${}", k, i + 1));
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
}
