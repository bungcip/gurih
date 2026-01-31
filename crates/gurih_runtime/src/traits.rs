use crate::context::RuntimeContext;
use crate::datastore::DataStore;
use async_trait::async_trait;
use gurih_ir::Schema;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;

#[async_trait]
pub trait DataAccess: Send + Sync {
    fn get_schema(&self) -> &Schema;
    fn datastore(&self) -> &Arc<dyn DataStore>;

    async fn create(&self, entity_name: &str, data: Value, ctx: &RuntimeContext) -> Result<String, String>;
    async fn read(&self, entity_name: &str, id: &str) -> Result<Option<Arc<Value>>, String>;
    async fn update(&self, entity_name: &str, id: &str, data: Value, ctx: &RuntimeContext) -> Result<(), String>;
    async fn delete(&self, entity_name: &str, id: &str, ctx: &RuntimeContext) -> Result<(), String>;
    async fn list(
        &self,
        entity: &str,
        limit: Option<usize>,
        offset: Option<usize>,
        filters: Option<HashMap<String, String>>,
    ) -> Result<Vec<Arc<Value>>, String>;
}
