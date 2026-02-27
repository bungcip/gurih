use async_trait::async_trait;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;

#[async_trait]
pub trait DataStore: Send + Sync {
    async fn insert(&self, entity: &str, record: Value) -> Result<String, String>;
    async fn insert_many(&self, entity: &str, records: Vec<Value>) -> Result<Vec<String>, String>;
    async fn get(&self, entity: &str, id: &str) -> Result<Option<Arc<Value>>, String>;
    async fn update(&self, entity: &str, id: &str, record: Value) -> Result<(), String>;
    async fn delete(&self, entity: &str, id: &str) -> Result<(), String>;
    async fn list(&self, entity: &str, limit: Option<usize>, offset: Option<usize>) -> Result<Vec<Arc<Value>>, String>;
    async fn find(&self, entity: &str, filters: HashMap<String, String>) -> Result<Vec<Arc<Value>>, String>;
    async fn find_first(&self, entity: &str, filters: HashMap<String, String>) -> Result<Option<Arc<Value>>, String>;
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
