use async_trait::async_trait;
use gurih_ir::{
    ActionStep, DatabaseSchema, DatabaseType, EntitySchema, Expression, FieldSchema, FieldType, PostingLineSchema,
    PostingRuleSchema, Schema, StateSchema, Symbol, Transition, TransitionEffect, WorkflowSchema,
};
use gurih_runtime::context::RuntimeContext;
use gurih_runtime::data::DataEngine;
use gurih_runtime::datastore::DataStore;
use gurih_runtime::errors::RuntimeError;
use gurih_runtime::plugins::Plugin;
use gurih_runtime::store::init_datastore;
use gurih_runtime::traits::DataAccess;
use serde_json::{Value, json};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

struct TriggerPlugin {
    rule_name: String,
}

#[async_trait]
impl Plugin for TriggerPlugin {
    fn name(&self) -> &str {
        "TriggerPlugin"
    }

    async fn check_precondition(
        &self,
        _name: &str,
        _args: &[Expression],
        _kwargs: &HashMap<String, String>,
        _entity_data: &Value,
        _schema: &Schema,
        _datastore: Option<&Arc<dyn DataStore>>,
    ) -> Result<(), RuntimeError> {
        Ok(())
    }

    async fn apply_effect(
        &self,
        name: &str,
        _args: &[Expression],
        _kwargs: &HashMap<String, String>,
        _schema: &Schema,
        _datastore: Option<&Arc<dyn DataStore>>,
        _entity_name: &str,
        _entity_data: &Value,
    ) -> Result<(Value, Vec<String>, Vec<Symbol>), RuntimeError> {
        if name == "trigger_posting" {
            return Ok((Value::Null, vec![], vec![Symbol::from(self.rule_name.clone())]));
        }
        Ok((Value::Null, vec![], vec![]))
    }

    async fn execute_action_step(
        &self,
        _step_name: &str,
        _step: &ActionStep,
        _params: &HashMap<String, String>,
        _data_access: &dyn DataAccess,
        _ctx: &RuntimeContext,
    ) -> Result<bool, RuntimeError> {
        Ok(false)
    }
}

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

fn create_schema(num_accounts: usize) -> Schema {
    let mut schema = Schema::default();

    // Define Account Entity
    let account = EntitySchema {
        name: Symbol::from("Account"),
        table_name: Symbol::from("account"),
        fields: vec![
            create_field("id", FieldType::Uuid, false, true),
            create_field("code", FieldType::String, true, true),
            create_field("name", FieldType::String, true, false),
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

    // Define SourceDoc with Workflow
    let source_doc = EntitySchema {
        name: Symbol::from("SourceDoc"),
        table_name: Symbol::from("source_doc"),
        fields: vec![
            create_field("id", FieldType::Uuid, false, true),
            create_field("status", FieldType::String, false, false),
            create_field("amount", FieldType::Money, false, false),
        ],
        relationships: vec![],
        options: HashMap::new(),
        seeds: None,
    };
    schema.entities.insert(Symbol::from("SourceDoc"), source_doc);

    // Workflow
    let workflow = WorkflowSchema {
        name: Symbol::from("DocFlow"),
        entity: Symbol::from("SourceDoc"),
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
        transitions: vec![Transition {
            name: Symbol::from("Post"),
            from: Symbol::from("Draft"),
            to: Symbol::from("Posted"),
            required_permission: None,
            preconditions: vec![],
            effects: vec![TransitionEffect::Custom {
                name: Symbol::from("trigger_posting"),
                args: vec![],
                kwargs: HashMap::new(),
            }],
        }],
    };
    schema.workflows.insert(Symbol::from("SourceDoc"), workflow);

    // Posting Rule
    let mut lines = Vec::new();
    for i in 0..num_accounts {
        lines.push(PostingLineSchema {
            account: Symbol::from(format!("ACC-{:03}", i)),
            debit_expr: Some(Expression::Literal(10.0)),
            credit_expr: None,
            fields: HashMap::new(),
        });
    }

    let rule = PostingRuleSchema {
        name: Symbol::from("PR1"),
        source_entity: Symbol::from("SourceDoc"),
        description_expr: Expression::StringLiteral("Posting".to_string()),
        date_expr: Expression::StringLiteral("2024-01-01".to_string()),
        lines,
        auto_post: false,
    };
    schema.posting_rules.insert(Symbol::from("PR1"), rule);

    // DB Config
    schema.database = Some(DatabaseSchema {
        db_type: DatabaseType::Sqlite,
        url: "sqlite::memory:".to_string(),
    });

    schema
}

#[tokio::test]
async fn bench_posting_rule_n_plus_1() {
    let num_accounts = 200; // Keep it moderate for fast test but enough to show trend
    let schema = Arc::new(create_schema(num_accounts));

    // Init Datastore
    let datastore = init_datastore(schema.clone(), None)
        .await
        .expect("Failed to init datastore");

    let engine = DataEngine::new(schema.clone(), datastore).with_plugins(vec![Box::new(TriggerPlugin {
        rule_name: "PR1".to_string(),
    })]);

    let ctx = RuntimeContext::system();

    // 1. Create Accounts
    println!("Creating {} accounts...", num_accounts);
    let mut account_data = Vec::new();
    for i in 0..num_accounts {
        account_data.push(json!({
            "code": format!("ACC-{:03}", i),
            "name": format!("Account {:03}", i)
        }));
    }
    engine
        .create_many("Account", account_data, &ctx)
        .await
        .expect("Failed to create accounts");

    // 2. Create Source Doc
    let doc_id = engine
        .create(
            "SourceDoc",
            json!({
                "status": "Draft",
                "amount": "1000.00"
            }),
            &ctx,
        )
        .await
        .expect("Failed to create doc");

    // 3. Update Doc to Posted -> Triggers Rule
    println!("Executing posting rule...");
    let start = Instant::now();

    engine
        .update(
            "SourceDoc",
            &doc_id,
            json!({
                "status": "Posted"
            }),
            &ctx,
        )
        .await
        .expect("Failed to post doc");

    let duration = start.elapsed();
    println!(
        "BENCH_RESULT: Posting rule with {} lines took: {:?}",
        num_accounts, duration
    );
}
