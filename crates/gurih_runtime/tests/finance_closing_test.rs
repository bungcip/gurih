use async_trait::async_trait;
use gurih_ir::{ActionStep, ActionStepType, EntitySchema, FieldSchema, FieldType, Schema, Symbol};
use gurih_plugins::finance::FinancePlugin;
use gurih_runtime::context::RuntimeContext;
use gurih_runtime::data::DataEngine;
use gurih_runtime::datastore::DataStore;
use gurih_runtime::plugins::Plugin;
use serde_json::{Value, json};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

// Mock DataStore with mutable state for checking creates
struct MockDataStore {
    pub created_journals: Arc<Mutex<Vec<Value>>>,
    pub created_lines: Arc<Mutex<Vec<Value>>>,
}

#[async_trait]
impl DataStore for MockDataStore {
    async fn insert(&self, entity: &str, data: Value) -> Result<String, String> {
        if entity == "journal_entry" {
            self.created_journals.lock().unwrap().push(data.clone());
            Ok(data
                .get("id")
                .and_then(|v| v.as_str())
                .unwrap_or("new_j_id")
                .to_string())
        } else if entity == "journal_line" {
            self.created_lines.lock().unwrap().push(data.clone());
            Ok("new_l_id".to_string())
        } else {
            Ok("1".to_string())
        }
    }

    async fn insert_many(&self, entity: &str, records: Vec<Value>) -> Result<Vec<String>, String> {
        let mut ids = Vec::new();
        for record in records {
            ids.push(self.insert(entity, record).await?);
        }
        Ok(ids)
    }

    async fn update(&self, _entity: &str, _id: &str, _data: Value) -> Result<(), String> {
        Ok(())
    }
    async fn delete(&self, _entity: &str, _id: &str) -> Result<(), String> {
        Ok(())
    }

    async fn get(&self, entity: &str, _id: &str) -> Result<Option<Arc<Value>>, String> {
        if entity == "accounting_period" {
            Ok(Some(Arc::new(json!({
                "id": "period1",
                "name": "2023",
                "start_date": "2023-01-01",
                "end_date": "2023-12-31",
                "status": "Open"
            }))))
        } else {
            Ok(None)
        }
    }

    async fn list(
        &self,
        _entity: &str,
        _limit: Option<usize>,
        _offset: Option<usize>,
    ) -> Result<Vec<Arc<Value>>, String> {
        Ok(vec![])
    }

    async fn find(&self, entity: &str, filters: HashMap<String, String>) -> Result<Vec<Arc<Value>>, String> {
        if entity == "Account" || entity == "account" {
            // case sensitivity depends on caller
            if filters.get("name").map(|s| s.as_str()) == Some("Retained Earnings")
                || filters.get("system_tag").map(|s| s.as_str()) == Some("retained_earnings")
            {
                return Ok(vec![Arc::new(json!({
                    "id": "retained_earnings_id",
                    "name": "Retained Earnings",
                    "type": "Equity"
                }))]);
            }
        }
        Ok(vec![])
    }

    async fn find_first(&self, entity: &str, filters: HashMap<String, String>) -> Result<Option<Arc<Value>>, String> {
        let res = self.find(entity, filters).await?;
        if res.is_empty() {
            Ok(None)
        } else {
            Ok(Some(res[0].clone()))
        }
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
        Ok(vec![])
    }

    async fn query_with_params(&self, sql: &str, _params: Vec<Value>) -> Result<Vec<Arc<Value>>, String> {
        // Mock the aggregation query
        if sql.contains("SUM(jl.debit)") {
            // Return mock balances
            // Rev: Credit 1000 (Revenue is Credit normal)
            // Exp: Debit 400 (Expense is Debit normal)
            // Profit = 600.
            // Expected Closing:
            // Debit Rev 1000
            // Credit Exp 400
            // Credit RE 600

            return Ok(vec![
                Arc::new(json!({
                    "account_id": "rev_id",
                    "total_debit": 0.0,
                    "total_credit": 1000.0,
                    "account_type": "Revenue"
                })),
                Arc::new(json!({
                    "account_id": "exp_id",
                    "total_debit": 400.0,
                    "total_credit": 0.0,
                    "account_type": "Expense"
                })),
            ]);
        }
        Ok(vec![])
    }
}

fn create_field(name: &str, ftype: FieldType) -> FieldSchema {
    FieldSchema {
        name: Symbol::from(name),
        field_type: ftype,
        required: false,
        unique: false,
        default: None,
        references: None,
        serial_generator: None,
        storage: None,
        resize: None,
        filetype: None,
    }
}

#[tokio::test]
async fn test_generate_closing_entry() {
    let mut schema = Schema::default();

    // Define Entities needed
    schema.entities.insert(
        Symbol::from("JournalEntry"),
        EntitySchema {
            name: Symbol::from("JournalEntry"),
            table_name: Symbol::from("journal_entry"),
            fields: vec![
                create_field("description", FieldType::String),
                create_field("date", FieldType::Date),
                create_field("status", FieldType::String),
            ],
            relationships: vec![],
            options: HashMap::new(),
            seeds: None,
        },
    );

    schema.entities.insert(
        Symbol::from("JournalLine"),
        EntitySchema {
            name: Symbol::from("JournalLine"),
            table_name: Symbol::from("journal_line"),
            fields: vec![
                create_field("account", FieldType::String),
                create_field("debit", FieldType::Money),
                create_field("credit", FieldType::Money),
                create_field("journal_entry", FieldType::String),
            ],
            relationships: vec![],
            options: HashMap::new(),
            seeds: None,
        },
    );

    schema.entities.insert(
        Symbol::from("AccountingPeriod"),
        EntitySchema {
            name: Symbol::from("AccountingPeriod"),
            table_name: Symbol::from("accounting_period"),
            fields: vec![],
            relationships: vec![],
            options: HashMap::new(),
            seeds: None,
        },
    );

    schema.entities.insert(
        Symbol::from("Account"),
        EntitySchema {
            name: Symbol::from("Account"),
            table_name: Symbol::from("account"),
            fields: vec![],
            relationships: vec![],
            options: HashMap::new(),
            seeds: None,
        },
    );

    let mock_ds = Arc::new(MockDataStore {
        created_journals: Arc::new(Mutex::new(vec![])),
        created_lines: Arc::new(Mutex::new(vec![])),
    });

    let engine = DataEngine::new(Arc::new(schema), mock_ds.clone()).with_plugins(vec![Box::new(FinancePlugin)]);

    let ctx = RuntimeContext::system();

    let step = ActionStep {
        step_type: ActionStepType::Custom("finance:generate_closing_entry".to_string()),
        target: Symbol::from(""),
        args: HashMap::from([("period_id".to_string(), "period1".to_string())]),
    };

    let plugin = FinancePlugin;
    let res = plugin
        .execute_action_step(
            "finance:generate_closing_entry",
            &step,
            &HashMap::new(),
            &engine, // DataEngine implements DataAccess
            &ctx,
        )
        .await;

    if let Err(e) = &res {
        println!("Error: {:?}", e);
    }
    assert!(res.is_ok());
    assert!(res.unwrap());

    let journals = mock_ds.created_journals.lock().unwrap();
    assert_eq!(journals.len(), 1);
    assert_eq!(journals[0].get("description").unwrap(), "Closing Entry for 2023");

    let lines = mock_ds.created_lines.lock().unwrap();
    // 3 lines: 1 for Rev, 1 for Exp, 1 for RE plug
    assert_eq!(lines.len(), 3);

    // Check Revenue Line (Debited 1000)
    let rev_line = lines.iter().find(|l| l.get("account").unwrap() == "rev_id").unwrap();
    let rev_debit: f64 = rev_line.get("debit").unwrap().as_str().unwrap().parse().unwrap();
    assert_eq!(rev_debit, 1000.0);

    // Check Expense Line (Credited 400)
    let exp_line = lines.iter().find(|l| l.get("account").unwrap() == "exp_id").unwrap();
    let exp_credit: f64 = exp_line.get("credit").unwrap().as_str().unwrap().parse().unwrap();
    assert_eq!(exp_credit, 400.0);

    // Check RE Plug (Credited 600)
    let re_line = lines
        .iter()
        .find(|l| l.get("account").unwrap() == "retained_earnings_id")
        .unwrap();
    let re_credit: f64 = re_line.get("credit").unwrap().as_str().unwrap().parse().unwrap();
    assert_eq!(re_credit, 600.0);
}
