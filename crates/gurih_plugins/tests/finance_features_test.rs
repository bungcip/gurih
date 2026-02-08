use async_trait::async_trait;
use gurih_ir::{
    Expression, QuerySchema, QuerySelection, QueryType, Schema, StateSchema, Symbol, Transition,
    TransitionPrecondition, WorkflowSchema,
};
use gurih_runtime::data::DataEngine;
use gurih_runtime::datastore::DataStore;
use gurih_runtime::query_engine::{QueryEngine, QueryPlan};
use gurih_runtime::workflow::WorkflowEngine;
use serde_json::{Value, json};
use std::collections::HashMap;
use std::sync::Arc;

// Mock DataStore
struct MockDataStore;
#[async_trait]
impl DataStore for MockDataStore {
    async fn insert(&self, _entity: &str, _data: Value) -> Result<String, String> {
        Ok("1".to_string())
    }
    async fn update(&self, _entity: &str, _id: &str, _data: Value) -> Result<(), String> {
        Ok(())
    }
    async fn delete(&self, _entity: &str, _id: &str) -> Result<(), String> {
        Ok(())
    }
    async fn get(&self, _entity: &str, _id: &str) -> Result<Option<Arc<Value>>, String> {
        // Return dummy record for "JournalEntry" "1" -> Status "Posted"
        Ok(Some(Arc::new(json!({
            "id": "1",
            "status": "Posted",
            "date": "2024-01-01"
        }))))
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
    async fn query(&self, _sql: &str) -> Result<Vec<Arc<Value>>, String> {
        // Return Open Period
        Ok(vec![Arc::new(json!({"id": 1}))])
    }
    async fn query_with_params(&self, _sql: &str, _params: Vec<Value>) -> Result<Vec<Arc<Value>>, String> {
        // Return Open Period
        Ok(vec![Arc::new(json!({"id": 1}))])
    }
}

#[tokio::test]
async fn test_immutability() {
    let mut schema = Schema::default();
    let entity_name = Symbol::from("JournalEntry");

    // Add Entity Schema (Minimal)
    schema.entities.insert(
        entity_name,
        gurih_ir::EntitySchema {
            name: entity_name,
            table_name: Symbol::from("journal_entry"),
            fields: vec![gurih_ir::FieldSchema {
                name: Symbol::from("status"),
                field_type: gurih_ir::FieldType::String,
                required: false,
                unique: false,
                default: None,
                references: None,
                serial_generator: None,
                storage: None,
                resize: None,
                filetype: None,
            }],
            relationships: vec![],
            options: std::collections::HashMap::new(),
            seeds: None,
        },
    );

    let workflow = WorkflowSchema {
        name: Symbol::from("JournalWF"),
        entity: entity_name,
        field: Symbol::from("status"),
        initial_state: Symbol::from("Draft"),
        states: vec![
            StateSchema {
                name: Symbol::from("Draft"),
                immutable: false,
            },
            StateSchema {
                name: Symbol::from("Posted"),
                immutable: true,
            },
        ],
        transitions: vec![],
    };
    schema.workflows.insert(workflow.name, workflow);

    let datastore = Arc::new(MockDataStore);
    let engine = DataEngine::new(Arc::new(schema), datastore);
    let ctx = gurih_runtime::context::RuntimeContext::system(); // Should have permissions

    // Update Posted Record (Immutable)
    // Try to update "description"
    let update_data = json!({
        "description": "Hacked"
    });

    let res = engine.update("JournalEntry", "1", update_data, &ctx).await;
    assert!(res.is_err());
    assert!(res.err().unwrap().contains("immutable"));
}

#[tokio::test]
async fn test_delete_immutable() {
    let mut schema = Schema::default();
    let entity_name = Symbol::from("JournalEntry");

    // Add Entity Schema (Minimal)
    schema.entities.insert(
        entity_name,
        gurih_ir::EntitySchema {
            name: entity_name,
            table_name: Symbol::from("journal_entry"),
            fields: vec![gurih_ir::FieldSchema {
                name: Symbol::from("status"),
                field_type: gurih_ir::FieldType::String,
                required: false,
                unique: false,
                default: None,
                references: None,
                serial_generator: None,
                storage: None,
                resize: None,
                filetype: None,
            }],
            relationships: vec![],
            options: std::collections::HashMap::new(),
            seeds: None,
        },
    );

    let workflow = WorkflowSchema {
        name: Symbol::from("JournalWF"),
        entity: entity_name,
        field: Symbol::from("status"),
        initial_state: Symbol::from("Draft"),
        states: vec![
            StateSchema {
                name: Symbol::from("Draft"),
                immutable: false,
            },
            StateSchema {
                name: Symbol::from("Posted"),
                immutable: true,
            },
        ],
        transitions: vec![],
    };
    schema.workflows.insert(workflow.name, workflow);

    let datastore = Arc::new(MockDataStore);
    let engine = DataEngine::new(Arc::new(schema), datastore);
    let ctx = gurih_runtime::context::RuntimeContext::system();

    // Delete Posted Record (Immutable)
    let res = engine.delete("JournalEntry", "1", &ctx).await;
    assert!(res.is_err(), "Should not be able to delete immutable record");
    assert!(res.err().unwrap().contains("immutable"));
}

#[tokio::test]
async fn test_period_open_configured() {
    // Test WorkflowEngine directly for PeriodOpen
    let engine = WorkflowEngine::new();
    let datastore: Arc<dyn DataStore> = Arc::new(MockDataStore);

    let pre = TransitionPrecondition::Custom {
        name: Symbol::from("period_open"),
        args: vec![Expression::StringLiteral("MyPeriod".to_string())],
        kwargs: HashMap::new(),
    };
    let data = json!({ "date": "2024-01-01" });

    // validate_transition calls check_precondition
    let mut schema = Schema::default();

    // Add "MyPeriod" entity
    schema.entities.insert(
        Symbol::from("MyPeriod"),
        gurih_ir::EntitySchema {
            name: Symbol::from("MyPeriod"),
            table_name: Symbol::from("my_period"),
            fields: vec![],
            relationships: vec![],
            options: std::collections::HashMap::new(),
            seeds: None,
        },
    );

    let wf = WorkflowSchema {
        name: Symbol::from("WF"),
        entity: Symbol::from("E"),
        field: Symbol::from("s"),
        initial_state: Symbol::from("A"),
        states: vec![
            StateSchema {
                name: Symbol::from("A"),
                immutable: false,
            },
            StateSchema {
                name: Symbol::from("B"),
                immutable: false,
            },
        ],
        transitions: vec![Transition {
            name: Symbol::from("T"),
            from: Symbol::from("A"),
            to: Symbol::from("B"),
            required_permission: None,
            preconditions: vec![pre],
            effects: vec![],
        }],
    };
    schema.workflows.insert(wf.name, wf);

    let res = engine
        .validate_transition(&schema, Some(&datastore), "E", "A", "B", &data)
        .await;
    assert!(res.is_ok());
}

#[test]
fn test_query_group_by() {
    let mut schema = Schema::default();
    schema.queries.insert(
        Symbol::from("Q"),
        QuerySchema {
            name: Symbol::from("Q"),
            params: vec![],
            root_entity: Symbol::from("E"),
            query_type: QueryType::Flat,
            selections: vec![QuerySelection {
                field: Symbol::from("f"),
                alias: None,
            }],
            formulas: vec![],
            filters: vec![],
            joins: vec![],
            group_by: vec![Symbol::from("f")],
            hierarchy: None,
        },
    );

    let runtime_params = std::collections::HashMap::new();
    let plan = QueryEngine::plan(&schema, "Q", &runtime_params).unwrap();
    let sql = if let QueryPlan::ExecuteSql { sql, .. } = &plan.plans[0] {
        sql
    } else {
        panic!("Expected ExecuteSql");
    };
    assert!(sql.contains("GROUP BY [f]"));
}
