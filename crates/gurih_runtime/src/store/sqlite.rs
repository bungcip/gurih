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

    fn build_where_clause(filters: &HashMap<String, String>) -> Result<(String, Vec<&String>), String> {
        if filters.is_empty() {
            return Ok((String::new(), vec![]));
        }

        let mut query = String::new();
        let mut params = vec![];

        let mut keys: Vec<&String> = filters.keys().collect();
        keys.sort();

        query.push_str(" WHERE ");
        for (i, k) in keys.into_iter().enumerate() {
            validate_identifier(k)?;
            if i > 0 {
                query.push_str(" AND ");
            }
            query.push_str(&format!("\"{}\" = ?", k));
            params.push(filters.get(k).unwrap());
        }
        Ok((query, params))
    }

    fn row_to_json(row: &sqlx::sqlite::SqliteRow) -> Value {
        use serde_json::Number;

        let mut map = serde_json::Map::new();
        for (i, col) in row.columns().iter().enumerate() {
            let name = col.name();
            let type_name = col.type_info().name();

            let val = match type_name {
                "TEXT" | "VARCHAR" | "CHAR" | "CLOB" => match row.try_get::<Option<String>, _>(i) {
                    Ok(Some(v)) => Value::String(v),
                    _ => Value::Null,
                },
                "INTEGER" | "INT" | "BIGINT" | "TINYINT" | "SMALLINT" => match row.try_get::<Option<i64>, _>(i) {
                    Ok(Some(v)) => Value::Number(Number::from(v)),
                    _ => Value::Null,
                },
                "REAL" | "FLOAT" | "DOUBLE" | "NUMERIC" => match row.try_get::<Option<f64>, _>(i) {
                    Ok(Some(v)) => Number::from_f64(v).map(Value::Number).unwrap_or(Value::Null),
                    _ => Value::Null,
                },
                "BOOLEAN" => match row.try_get::<Option<bool>, _>(i) {
                    Ok(Some(v)) => Value::Bool(v),
                    _ => Value::Null,
                },
                "DATE" => {
                    // SQLite doesn't have a native DATE type, but we might have declared it as DATE.
                    // SQLx maps DATE to chrono::NaiveDate if the column type is "DATE".
                    let v: Option<NaiveDate> = row.try_get(i).unwrap_or(None);
                    serde_json::to_value(v).unwrap_or(Value::Null)
                }
                "DATETIME" | "TIMESTAMP" => {
                    let v: Option<chrono::NaiveDateTime> = row.try_get(i).unwrap_or(None);
                    serde_json::to_value(v).unwrap_or(Value::Null)
                }
                _ => {
                    // Fallback: try f64 first, then String
                    if let Ok(val) = row.try_get::<Option<f64>, _>(i) {
                        if let Some(v) = val {
                            Number::from_f64(v).map(Value::Number).unwrap_or(Value::Null)
                        } else {
                            Value::Null
                        }
                    } else {
                        match row.try_get::<Option<String>, _>(i) {
                            Ok(Some(v)) => Value::String(v),
                            _ => Value::Null,
                        }
                    }
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
        let mut obj = record.as_object().ok_or("Record must be object")?.clone();

        // Generate ID if missing
        if !obj.contains_key("id") {
            obj.insert("id".to_string(), Value::String(uuid::Uuid::new_v4().to_string()));
        }

        // Estimate capacity to reduce reallocations
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

    async fn insert_many(&self, entity: &str, records: Vec<Value>) -> Result<Vec<String>, String> {
        validate_identifier(entity)?;
        if records.is_empty() {
            return Ok(vec![]);
        }

        // Prepare IDs and ensure objects
        let mut prepared_records = Vec::with_capacity(records.len());
        let mut ids = Vec::with_capacity(records.len());

        for record in records {
            let mut obj = record.as_object().ok_or("Record must be object")?.clone();
            if !obj.contains_key("id") {
                let new_id = uuid::Uuid::new_v4().to_string();
                obj.insert("id".to_string(), Value::String(new_id.clone()));
                ids.push(new_id);
            } else {
                let id = obj.get("id").and_then(|v| v.as_str()).unwrap_or_default().to_string();
                ids.push(id);
            }
            prepared_records.push(obj);
        }

        // We use the keys from the first record as the schema for the batch.
        let keys: Vec<String> = prepared_records[0].keys().cloned().collect();
        let cols_count = keys.len();

        // Chunking: SQLite limit is typically 999 parameters.
        let chunk_size = if cols_count > 0 { 900 / cols_count } else { 100 };
        let chunk_size = std::cmp::max(1, chunk_size);

        for chunk in prepared_records.chunks(chunk_size) {
            let mut query = String::new();
            query.push_str("INSERT INTO \"");
            query.push_str(entity);
            query.push_str("\" (");

            for (i, key) in keys.iter().enumerate() {
                validate_identifier(key)?;
                if i > 0 {
                    query.push_str(", ");
                }
                query.push('"');
                query.push_str(key);
                query.push('"');
            }
            query.push_str(") VALUES ");

            let mut params = Vec::new();

            for (r_idx, record) in chunk.iter().enumerate() {
                if r_idx > 0 {
                    query.push_str(", ");
                }
                query.push('(');
                for (c_idx, key) in keys.iter().enumerate() {
                    if c_idx > 0 {
                        query.push_str(", ");
                    }
                    query.push('?');
                    params.push(record.get(key).unwrap_or(&Value::Null));
                }
                query.push(')');
            }

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

            q.execute(&self.pool).await.map_err(|e| e.to_string())?;
        }

        Ok(ids)
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

        let (where_clause, params) = Self::build_where_clause(&filters)?;
        query.push_str(&where_clause);

        let mut q = sqlx::query(&query);
        for p in params {
            q = q.bind(p);
        }

        let rows = q.fetch_all(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(rows.iter().map(|r| Arc::new(Self::row_to_json(r))).collect())
    }

    async fn find_first(&self, entity: &str, filters: HashMap<String, String>) -> Result<Option<Arc<Value>>, String> {
        validate_identifier(entity)?;
        let mut query = format!("SELECT * FROM \"{}\"", entity);

        let (where_clause, params) = Self::build_where_clause(&filters)?;
        query.push_str(&where_clause);

        query.push_str(" LIMIT 1");

        let mut q = sqlx::query(&query);
        for p in params {
            q = q.bind(p);
        }

        let row = q.fetch_optional(&self.pool).await.map_err(|e| e.to_string())?;
        if let Some(r) = row {
            Ok(Some(Arc::new(Self::row_to_json(&r))))
        } else {
            Ok(None)
        }
    }

    async fn count(&self, entity: &str, filters: HashMap<String, String>) -> Result<i64, String> {
        validate_identifier(entity)?;
        let mut query = format!("SELECT COUNT(*) FROM \"{}\"", entity);

        let (where_clause, params) = Self::build_where_clause(&filters)?;
        query.push_str(&where_clause);

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

        let (where_clause, params) = Self::build_where_clause(&filters)?;
        query.push_str(&where_clause);

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

    async fn query_with_params(&self, sql: &str, params: Vec<Value>) -> Result<Vec<Arc<Value>>, String> {
        let mut q = sqlx::query(sql);
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
        let rows = q.fetch_all(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(rows.iter().map(|r| Arc::new(Self::row_to_json(r))).collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_build_where_clause_determinism() {
        let mut filters = HashMap::new();
        filters.insert("z".to_string(), "1".to_string());
        filters.insert("a".to_string(), "2".to_string());
        filters.insert("m".to_string(), "3".to_string());
        filters.insert("b".to_string(), "4".to_string());

        // Call the private method
        let (clause, params) = SqliteDataStore::build_where_clause(&filters).expect("Failed to build clause");

        // Check SQL string order
        // With sorted keys: a, b, m, z
        let expected = " WHERE \"a\" = ? AND \"b\" = ? AND \"m\" = ? AND \"z\" = ?";
        assert_eq!(clause, expected);

        // Check params order matches keys
        let expected_params = vec!["2", "4", "3", "1"];
        assert_eq!(params, expected_params);

        // Ensure deterministic regardless of insertion order
        let mut filters2 = HashMap::new();
        filters2.insert("a".to_string(), "2".to_string());
        filters2.insert("b".to_string(), "4".to_string());
        filters2.insert("m".to_string(), "3".to_string());
        filters2.insert("z".to_string(), "1".to_string());

        let (clause2, params2) = SqliteDataStore::build_where_clause(&filters2).expect("Failed to build clause");
        assert_eq!(clause, clause2);
        assert_eq!(params, params2);
    }
}
