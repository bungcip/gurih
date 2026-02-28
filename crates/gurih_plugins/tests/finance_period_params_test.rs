use async_trait::async_trait;
use gurih_ir::{EntitySchema, Expression, Schema, Symbol};
use gurih_plugins::finance::FinancePlugin;
use gurih_runtime::datastore::DataStore;
use gurih_runtime::plugins::Plugin;
use serde_json::{Value, json};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

// Mock DataStore that spies on query calls
struct SpyDataStore {
    pub query_params_calls: Arc<Mutex<Vec<(String, Vec<Value>)>>>,
    pub query_raw_calls: Arc<Mutex<Vec<String>>>,
}

impl SpyDataStore {
    fn new() -> Self {
        Self {
            query_params_calls: Arc::new(Mutex::new(Vec::new())),
            query_raw_calls: Arc::new(Mutex::new(Vec::new())),
        }
    }
}

#[async_trait]
impl DataStore for SpyDataStore {
    async fn insert(&self, _entity: &str, _data: Value) -> Result<String, String> {
        Ok("1".to_string())
    }
    async fn insert_many(&self, _entity: &str, _records: Vec<Value>) -> Result<Vec<String>, String> {
        Ok(vec![])
    }
    async fn update(&self, _entity: &str, _id: &str, _data: Value) -> Result<(), String> {
        Ok(())
    }
    async fn delete(&self, _entity: &str, _id: &str) -> Result<(), String> {
        Ok(())
    }
    async fn get(&self, _entity: &str, _id: &str) -> Result<Option<Arc<Value>>, String> {
        Ok(None)
    }
    async fn list(
        &self,
        _entity: &str,
        _limit: Option<usize>,
        _offset: Option<usize>,
    ) -> Result<Vec<Arc<Value>>, String> {
        Ok(vec![])
    }
    async fn find(&self, _entity: &str, _filters: HashMap<String, String>) -> Result<Vec<Arc<Value>>, String> {
        Ok(vec![])
    }
    async fn find_first(&self, _entity: &str, _filters: HashMap<String, String>) -> Result<Option<Arc<Value>>, String> {
        Ok(None)
    }
    async fn count(&self, _entity: &str, _filters: HashMap<String, String>) -> Result<i64, String> {
        Ok(0)
    }
    async fn aggregate(
        &self,
        _entity: &str,
        _group_by: &str,
        _filters: HashMap<String, String>,
    ) -> Result<Vec<(String, i64)>, String> {
        Ok(vec![])
    }
    async fn query(&self, sql: &str) -> Result<Vec<Arc<Value>>, String> {
        self.query_raw_calls.lock().unwrap().push(sql.to_string());
        // Return valid response to mimic open period
        Ok(vec![Arc::new(json!({"id": "period1"}))])
    }
    async fn query_with_params(&self, sql: &str, params: Vec<Value>) -> Result<Vec<Arc<Value>>, String> {
        self.query_params_calls.lock().unwrap().push((sql.to_string(), params));
        // Return valid response to mimic open period
        Ok(vec![Arc::new(json!({"id": "period1"}))])
    }
}

#[tokio::test]
async fn test_period_check_uses_params() {
    let spy = Arc::new(SpyDataStore::new());
    let datastore: Arc<dyn DataStore> = spy.clone();

    // Setup Schema
    let mut schema = Schema::default();
    let entity_name = Symbol::from("AccountingPeriod");
    schema.entities.insert(
        entity_name,
        EntitySchema {
            name: entity_name,
            table_name: Symbol::from("accounting_period"),
            fields: vec![],
            relationships: vec![],
            options: HashMap::new(),
            seeds: None,
        },
    );

    // Setup Plugin
    let plugin = FinancePlugin;

    // Call check_precondition("period_open", args=["AccountingPeriod"])
    let args = vec![Expression::StringLiteral("AccountingPeriod".to_string())];
    let kwargs = HashMap::new();
    let entity_data = json!({ "date": "2024-01-15" });

    let result = plugin
        .check_precondition("period_open", &args, &kwargs, &entity_data, &schema, Some(&datastore))
        .await;

    // 1. Verify success
    assert!(result.is_ok(), "period_open check should succeed: {:?}", result.err());

    // 2. Verify query_with_params was called
    let params_calls = spy.query_params_calls.lock().unwrap();
    let raw_calls = spy.query_raw_calls.lock().unwrap();

    assert_eq!(raw_calls.len(), 0, "Should not call raw query()");
    assert_eq!(params_calls.len(), 1, "Should call query_with_params()");

    // 3. Verify SQL and parameters
    let (sql, params) = &params_calls[0];

    // Check SQL has placeholders
    assert!(sql.contains("WHERE status = 'Open'"), "SQL missing status check");
    // Depending on DB type (default sqlite), placeholder is ?
    assert!(sql.contains("start_date <= ?"), "SQL missing start_date placeholder");
    assert!(sql.contains("end_date >= ?"), "SQL missing end_date placeholder");

    // Check Params
    assert_eq!(params.len(), 2, "Should have 2 parameters");
    assert_eq!(params[0], Value::String("2024-01-15".to_string()));
    assert_eq!(params[1], Value::String("2024-01-15".to_string()));
}
