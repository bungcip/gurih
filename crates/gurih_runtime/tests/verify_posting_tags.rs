use gurih_ir::{
    DatabaseType, EntitySchema, Expression, FieldSchema, FieldType, PostingLineSchema,
    PostingRuleSchema, Schema, Symbol, StateSchema, Transition, TransitionEffect, WorkflowSchema
};
use gurih_runtime::context::RuntimeContext;
use gurih_runtime::data::DataEngine;
use gurih_runtime::datastore::MemoryDataStore;
use gurih_runtime::datastore::DataStore;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::Arc;

fn create_field(name: &str, field_type: FieldType, required: bool, unique: bool) -> FieldSchema {
    FieldSchema {
        name: Symbol::from(name),
        field_type,
        required,
        unique,
        default: None,
        references: None,
        serial_generator: None,
        storage: None,
        resize: None,
        filetype: None,
    }
}

fn create_test_schema() -> Schema {
    let mut schema = Schema::default();

    // Define Account Entity with system_tag
    let account = EntitySchema {
        name: Symbol::from("Account"),
        table_name: Symbol::from("account"),
        fields: vec![
            create_field("id", FieldType::Uuid, false, true),
            create_field("code", FieldType::String, true, true),
            create_field("name", FieldType::String, true, false),
            create_field("system_tag", FieldType::String, false, true),
        ],
        relationships: vec![],
        options: HashMap::new(),
        seeds: None,
    };
    schema.entities.insert(Symbol::from("Account"), account);

    // Define JournalEntry
    let journal_entry = EntitySchema {
        name: Symbol::from("JournalEntry"),
        table_name: Symbol::from("journal_entry"),
        fields: vec![
            create_field("id", FieldType::Uuid, false, true),
            create_field("description", FieldType::String, false, false),
            create_field("date", FieldType::Date, false, false),
            create_field("status", FieldType::String, false, false),
        ],
        relationships: vec![],
        options: HashMap::new(),
        seeds: None,
    };
    schema.entities.insert(Symbol::from("JournalEntry"), journal_entry);

    // Define JournalLine
    let journal_line = EntitySchema {
        name: Symbol::from("JournalLine"),
        table_name: Symbol::from("journal_line"),
        fields: vec![
            create_field("id", FieldType::Uuid, false, true),
            {
                let mut f = create_field("journal_entry", FieldType::Relation, true, false);
                f.references = Some(Symbol::from("JournalEntry"));
                f
            },
            {
                let mut f = create_field("account", FieldType::Relation, true, false);
                f.references = Some(Symbol::from("Account"));
                f
            },
            create_field("debit", FieldType::Money, false, false),
            create_field("credit", FieldType::Money, false, false),
        ],
        relationships: vec![],
        options: HashMap::new(),
        seeds: None,
    };
    schema.entities.insert(Symbol::from("JournalLine"), journal_line);

    // Define SourceDoc
    let source_doc = EntitySchema {
        name: Symbol::from("SourceDoc"),
        table_name: Symbol::from("source_doc"),
        fields: vec![
             create_field("id", FieldType::Uuid, false, true),
             create_field("status", FieldType::String, false, false),
        ],
        relationships: vec![],
        options: HashMap::new(),
        seeds: None,
    };
    schema.entities.insert(Symbol::from("SourceDoc"), source_doc);

    // DB Config
    schema.database = Some(gurih_ir::DatabaseSchema {
        db_type: DatabaseType::Sqlite,
        url: "sqlite::memory:".to_string(),
    });

    // Define Posting Rule using system_tag
    let rule = PostingRuleSchema {
        name: Symbol::from("TestRule"),
        source_entity: Symbol::from("SourceDoc"),
        description_expr: Expression::StringLiteral("Test Posting".to_string()),
        date_expr: Expression::StringLiteral("2024-01-01".to_string()),
        lines: vec![
            PostingLineSchema {
                account: Symbol::from("cash_account"), // Matches system_tag
                debit_expr: Some(Expression::Literal(100.0)),
                credit_expr: None,
            },
            PostingLineSchema {
                account: Symbol::from("revenue_account"), // Matches system_tag
                debit_expr: None,
                credit_expr: Some(Expression::Literal(100.0)),
            },
        ],
    };
    schema.posting_rules.insert(Symbol::from("TestRule"), rule);

    // Workflow to trigger rule
    let workflow = WorkflowSchema {
        name: Symbol::from("DocFlow"),
        entity: Symbol::from("SourceDoc"),
        field: Symbol::from("status"),
        initial_state: Symbol::from("Draft"),
        states: vec![
            StateSchema { name: Symbol::from("Draft"), immutable: false },
            StateSchema { name: Symbol::from("Posted"), immutable: true },
        ],
        transitions: vec![
            Transition {
                name: Symbol::from("Post"),
                from: Symbol::from("Draft"),
                to: Symbol::from("Posted"),
                required_permission: None,
                preconditions: vec![],
                effects: vec![
                    TransitionEffect::Custom {
                        name: Symbol::from("trigger_posting"),
                        args: vec![],
                        kwargs: HashMap::new(),
                    }
                ],
            }
        ],
    };
    schema.workflows.insert(Symbol::from("SourceDoc"), workflow);

    schema
}

// Minimal Plugin to trigger posting
struct TriggerPlugin {
    rule_name: String,
}

#[async_trait::async_trait]
impl gurih_runtime::plugins::Plugin for TriggerPlugin {
    fn name(&self) -> &str { "TriggerPlugin" }

    async fn check_precondition(
        &self,
        _name: &str,
        _args: &[Expression],
        _kwargs: &HashMap<String, String>,
        _entity_data: &Value,
        _schema: &Schema,
        _datastore: Option<&Arc<dyn gurih_runtime::datastore::DataStore>>,
    ) -> Result<(), gurih_runtime::errors::RuntimeError> {
        Ok(())
    }

    async fn apply_effect(
        &self,
        name: &str,
        _args: &[Expression],
        _kwargs: &HashMap<String, String>,
        _schema: &Schema,
        _datastore: Option<&Arc<dyn gurih_runtime::datastore::DataStore>>,
        _entity_name: &str,
        _entity_data: &Value,
    ) -> Result<(Value, Vec<String>, Vec<Symbol>), gurih_runtime::errors::RuntimeError> {
        if name == "trigger_posting" {
            // Return rule name in postings list (3rd element)
            return Ok((Value::Null, vec![], vec![Symbol::from(self.rule_name.clone())]));
        }
        Ok((Value::Null, vec![], vec![]))
    }

    async fn execute_action_step(
        &self, _n: &str, _s: &gurih_ir::ActionStep, _p: &HashMap<String, String>, _d: &dyn gurih_runtime::traits::DataAccess, _c: &RuntimeContext
    ) -> Result<bool, gurih_runtime::errors::RuntimeError> { Ok(false) }
}

#[tokio::test]
async fn test_posting_rule_system_tag_lookup_full() {
    let schema = Arc::new(create_test_schema());
    let datastore = Arc::new(MemoryDataStore::new());

    // Register Plugin
    let engine = DataEngine::new(schema.clone(), datastore.clone())
        .with_plugins(vec![Box::new(TriggerPlugin { rule_name: "TestRule".to_string() })]);

    let ctx = RuntimeContext::system();

    // 1. Create Accounts with system_tags
    let accounts = vec![
        json!({
            "code": "101",
            "name": "Cash on Hand",
            "system_tag": "cash_account"
        }),
        json!({
            "code": "401",
            "name": "Sales Revenue",
            "system_tag": "revenue_account"
        }),
    ];
    let ids = engine
        .create_many("Account", accounts, &ctx)
        .await
        .expect("Failed to create accounts");

    // We don't know the order of IDs returned by create_many (it returns Vec<String>).
    // So we fetch them back to identify which is which.
    let accs = engine.list("Account", None, None, None, &ctx).await.expect("list accounts");
    let mut cash_id = "";
    let mut rev_id = "";

    for acc in &accs {
        let tag = acc.get("system_tag").and_then(|v| v.as_str()).unwrap_or("");
        if tag == "cash_account" {
            cash_id = acc.get("id").and_then(|v| v.as_str()).unwrap();
        } else if tag == "revenue_account" {
            rev_id = acc.get("id").and_then(|v| v.as_str()).unwrap();
        }
    }

    assert!(!cash_id.is_empty());
    assert!(!rev_id.is_empty());

    // 2. Create Source Doc
    let doc_id = engine.create("SourceDoc", json!({ "status": "Draft" }), &ctx).await.expect("Create doc");

    // 3. Update to Posted -> Triggers Rule -> Calls execute_posting_rule
    engine.update("SourceDoc", &doc_id, json!({ "status": "Posted" }), &ctx).await.expect("Post doc");

    // 4. Verify Journal Entry
    let journals = engine.list("JournalEntry", None, None, None, &ctx).await.expect("List journals");
    assert_eq!(journals.len(), 1, "Should create 1 journal entry");
    let je_id = journals[0].get("id").and_then(|v| v.as_str()).unwrap();

    // 5. Verify Journal Lines
    let mut filters = HashMap::new();
    filters.insert("journal_entry".to_string(), je_id.to_string());
    let lines = engine.list("JournalLine", None, None, Some(filters), &ctx).await.expect("List lines");

    assert_eq!(lines.len(), 2, "Should create 2 lines");

    // Verify account linking
    let mut found_cash = false;
    let mut found_rev = false;

    for line in lines {
        let acc_id = line.get("account").and_then(|v| v.as_str()).unwrap();
        if acc_id == cash_id {
            found_cash = true;
            assert_eq!(line.get("debit").and_then(|v| v.as_str()).unwrap(), "100.0");
        } else if acc_id == rev_id {
            found_rev = true;
            assert_eq!(line.get("credit").and_then(|v| v.as_str()).unwrap(), "100.0");
        }
    }

    assert!(found_cash, "Should find line linked to Cash account via system_tag");
    assert!(found_rev, "Should find line linked to Revenue account via system_tag");
}
