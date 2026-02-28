use async_trait::async_trait;
use gurih_ir::{ActionStep, ActionStepType, EntitySchema, Schema, Symbol};
use gurih_plugins::finance::FinancePlugin;
use gurih_runtime::context::RuntimeContext;
use gurih_runtime::data::DataEngine;
use gurih_runtime::datastore::DataStore;
use gurih_runtime::plugins::Plugin;
use serde_json::{Value, json};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

// Mock DataStore
#[derive(Clone)]
struct MockDataStore {
    pub lines: Arc<Mutex<HashMap<String, Value>>>,
    pub statuses: Arc<Mutex<HashMap<String, Value>>>,
    pub reconciliations: Arc<Mutex<Vec<Value>>>,
}

#[async_trait]
impl DataStore for MockDataStore {
    async fn insert(&self, entity: &str, data: Value) -> Result<String, String> {
        let id = data.get("id").and_then(|v| v.as_str()).unwrap_or("gen_id").to_string();
        if entity == "Reconciliation" || entity == "reconciliation" {
            self.reconciliations.lock().unwrap().push(data.clone());
        } else if entity == "JournalLineStatus" || entity == "journal_line_status" {
            self.statuses.lock().unwrap().insert(id.clone(), data.clone());
        }
        Ok(id)
    }

    async fn insert_many(&self, entity: &str, records: Vec<Value>) -> Result<Vec<String>, String> {
        let mut ids = Vec::new();
        for record in records {
            ids.push(self.insert(entity, record).await?);
        }
        Ok(ids)
    }

    async fn update(&self, entity: &str, id: &str, data: Value) -> Result<(), String> {
        if entity == "JournalLineStatus" || entity == "journal_line_status" {
            let mut statuses = self.statuses.lock().unwrap();
            if let Some(existing) = statuses.get_mut(id)
                && let Some(obj) = existing.as_object_mut()
                && let Some(update_obj) = data.as_object()
            {
                for (k, v) in update_obj {
                    obj.insert(k.clone(), v.clone());
                }
            }
        }
        Ok(())
    }

    async fn delete(&self, _entity: &str, _id: &str) -> Result<(), String> {
        Ok(())
    }

    async fn get(&self, entity: &str, id: &str) -> Result<Option<Arc<Value>>, String> {
        if entity == "JournalLine" || entity == "journal_line" {
            let lines = self.lines.lock().unwrap();
            return Ok(lines.get(id).map(|v| Arc::new(v.clone())));
        }
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

    async fn find(&self, entity: &str, filters: HashMap<String, String>) -> Result<Vec<Arc<Value>>, String> {
        if (entity == "JournalLine" || entity == "journal_line")
            && let Some(jid) = filters.get("journal_entry")
        {
            let lines = self.lines.lock().unwrap();
            let mut res = vec![];
            for val in lines.values() {
                if val.get("journal_entry").and_then(|v| v.as_str()) == Some(jid) {
                    res.push(Arc::new(val.clone()));
                }
            }
            return Ok(res);
        }
        if (entity == "JournalLineStatus" || entity == "journal_line_status")
            && let Some(lid) = filters.get("journal_line")
        {
            let statuses = self.statuses.lock().unwrap();
            for val in statuses.values() {
                if val.get("journal_line").and_then(|v| v.as_str()) == Some(lid) {
                    return Ok(vec![Arc::new(val.clone())]);
                }
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
    async fn query_with_params(&self, _sql: &str, _params: Vec<Value>) -> Result<Vec<Arc<Value>>, String> {
        Ok(vec![])
    }
}

#[tokio::test]
async fn test_finance_reconciliation() {
    let mock_ds = Arc::new(MockDataStore {
        lines: Arc::new(Mutex::new(HashMap::new())),
        statuses: Arc::new(Mutex::new(HashMap::new())),
        reconciliations: Arc::new(Mutex::new(vec![])),
    });

    // Setup Schema
    let mut schema = Schema::default();
    schema.entities.insert(
        Symbol::from("JournalLine"),
        EntitySchema {
            name: Symbol::from("JournalLine"),
            table_name: Symbol::from("journal_line"),
            fields: vec![],
            relationships: vec![],
            options: HashMap::new(),
            seeds: None,
        },
    );
    schema.entities.insert(
        Symbol::from("JournalLineStatus"),
        EntitySchema {
            name: Symbol::from("JournalLineStatus"),
            table_name: Symbol::from("journal_line_status"),
            fields: vec![],
            relationships: vec![],
            options: HashMap::new(),
            seeds: None,
        },
    );
    schema.entities.insert(
        Symbol::from("Reconciliation"),
        EntitySchema {
            name: Symbol::from("Reconciliation"),
            table_name: Symbol::from("reconciliation"),
            fields: vec![],
            relationships: vec![],
            options: HashMap::new(),
            seeds: None,
        },
    );

    // Setup Data
    // Invoice Line (Debit 100)
    let inv_line = json!({
        "id": "inv_line_1",
        "journal_entry": "inv_je_1",
        "account": "ar_acc",
        "party_id": "cust_1",
        "debit": "100.00",
        "credit": "0.00"
    });
    mock_ds
        .lines
        .lock()
        .unwrap()
        .insert("inv_line_1".to_string(), inv_line.clone());

    // Payment Line (Credit 60)
    let pay_line = json!({
        "id": "pay_line_1",
        "journal_entry": "pay_je_1",
        "account": "ar_acc",
        "party_id": "cust_1",
        "debit": "0.00",
        "credit": "60.00"
    });
    mock_ds
        .lines
        .lock()
        .unwrap()
        .insert("pay_line_1".to_string(), pay_line.clone());

    let plugin = FinancePlugin;
    let engine = DataEngine::new(Arc::new(schema.clone()), mock_ds.clone());
    let ctx = RuntimeContext::system();

    // 1. Init Status for Invoice
    let args = vec![]; // no args needed
    let mock_ds_dyn: Arc<dyn DataStore> = mock_ds.clone();
    plugin
        .apply_effect(
            "init_line_status",
            &args,
            &HashMap::new(),
            &schema,
            Some(&mock_ds_dyn),
            "JournalEntry",
            &json!({"id": "inv_je_1"}),
        )
        .await
        .unwrap();

    // Verify Invoice Status
    {
        let statuses = mock_ds.statuses.lock().unwrap();
        let s = statuses.values().find(|v| v["journal_line"] == "inv_line_1").unwrap();
        assert_eq!(s["amount_residual"], "100.00"); // 100 - 0
        assert_eq!(s["is_fully_reconciled"], false);
    }

    // 2. Init Status for Payment
    plugin
        .apply_effect(
            "init_line_status",
            &args,
            &HashMap::new(),
            &schema,
            Some(&mock_ds_dyn),
            "JournalEntry",
            &json!({"id": "pay_je_1"}),
        )
        .await
        .unwrap();

    // Verify Payment Status
    {
        let statuses = mock_ds.statuses.lock().unwrap();
        let s = statuses.values().find(|v| v["journal_line"] == "pay_line_1").unwrap();
        assert_eq!(s["amount_residual"], "60.00"); // abs(0 - 60)
    }

    // 3. Reconcile 60
    let step = ActionStep {
        step_type: ActionStepType::Custom("finance:reconcile_entries".to_string()),
        target: Symbol::from(""),
        args: HashMap::from([
            ("debit_line_id".to_string(), "inv_line_1".to_string()),
            ("credit_line_id".to_string(), "pay_line_1".to_string()),
            ("amount".to_string(), "60.00".to_string()),
        ]),
    };

    let res = plugin
        .execute_action_step("finance:reconcile_entries", &step, &HashMap::new(), &engine, &ctx)
        .await;

    if let Err(e) = &res {
        println!("Error: {:?}", e);
    }
    assert!(res.is_ok());

    // 4. Verify Residuals
    {
        let statuses = mock_ds.statuses.lock().unwrap();

        // Invoice: 100 - 60 = 40
        let inv_s = statuses.values().find(|v| v["journal_line"] == "inv_line_1").unwrap();
        assert_eq!(inv_s["amount_residual"], "40.00");
        assert_eq!(inv_s["is_fully_reconciled"], false);

        // Payment: 60 - 60 = 0
        let pay_s = statuses.values().find(|v| v["journal_line"] == "pay_line_1").unwrap();
        assert_eq!(pay_s["amount_residual"], "0.00");
        assert_eq!(pay_s["is_fully_reconciled"], true);
    }

    // 5. Verify Reconciliation Record
    {
        let recs = mock_ds.reconciliations.lock().unwrap();
        assert_eq!(recs.len(), 1);
        assert_eq!(recs[0]["amount"], "60.00");
        assert_eq!(recs[0]["debit_line"], "inv_line_1");
        assert_eq!(recs[0]["credit_line"], "pay_line_1");
    }
}
