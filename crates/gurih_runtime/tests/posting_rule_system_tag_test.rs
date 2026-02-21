use gurih_dsl::compiler::compile;
use gurih_runtime::context::RuntimeContext;
use gurih_runtime::data::DataEngine;
use gurih_runtime::datastore::{DataStore, MemoryDataStore};
use serde_json::json;
use std::sync::Arc;

#[tokio::test]
async fn test_posting_rule_system_tag() {
    let kdl = r#"
    entity "Account" {
        field:pk id
        field:string "code" unique=#true
        field:string "name" unique=#true
        field:string "system_tag" unique=#true required=#false
        field:string "type"
    }

    entity "JournalEntry" {
        field:pk id
        field:string "description"
        field:string "date"
        field:string "status" default="Draft"
        has_many "lines" "JournalLine" type="composition"
    }

    entity "JournalLine" {
        field:pk id
        belongs_to "JournalEntry"
        belongs_to "Account"
        field:money "debit" default=0
        field:money "credit" default=0
    }

    entity "Invoice" {
        field:pk id
        field:string "invoice_number"
        field:date "date"
        field:money "total_amount"
        field:string "status" default="Draft"
    }

    workflow "InvoiceWorkflow" for="Invoice" field="status" {
        state "Draft" initial=#true
        state "Posted"

        transition "post" {
            from "Draft"
            to "Posted"
            effects {
                post_journal "InvoicePosting"
            }
        }
    }

    posting_rule "InvoicePosting" for="Invoice" {
        description "\"Invoice #\" + doc.invoice_number"
        date "doc.date"

        entry {
            account "accounts_receivable" // Using system_tag
            debit "doc.total_amount"
        }

        entry {
            account "sales_revenue" // Using system_tag
            credit "doc.total_amount"
        }
    }
    "#;

    // 1. Compile Schema
    let schema = compile(kdl, None).expect("Failed to compile schema");
    let schema_arc = Arc::new(schema);

    // 2. Setup Runtime
    let datastore: Arc<dyn DataStore> = Arc::new(MemoryDataStore::new());
    let engine = DataEngine::new(schema_arc.clone(), datastore.clone());

    use async_trait::async_trait;
    use gurih_ir::{ActionStep, Expression, Schema, Symbol};
    use gurih_runtime::errors::RuntimeError;
    use gurih_runtime::plugins::Plugin;
    use gurih_runtime::traits::DataAccess;
    use std::collections::HashMap;

    struct MockFinancePlugin;

    #[async_trait]
    impl Plugin for MockFinancePlugin {
        fn name(&self) -> &str {
            "MockFinancePlugin"
        }
        async fn check_precondition(
            &self,
            _: &str,
            _: &[Expression],
            _: &HashMap<String, String>,
            _: &serde_json::Value,
            _: &Schema,
            _: Option<&Arc<dyn DataStore>>,
        ) -> Result<(), RuntimeError> {
            Ok(())
        }
        async fn apply_effect(
            &self,
            name: &str,
            args: &[Expression],
            _: &HashMap<String, String>,
            _: &Schema,
            _: Option<&Arc<dyn DataStore>>,
            _: &str,
            _: &serde_json::Value,
        ) -> Result<(serde_json::Value, Vec<String>, Vec<Symbol>), RuntimeError> {
            if name == "post_journal" {
                if let Some(Expression::StringLiteral(rule)) = args.first() {
                    return Ok((serde_json::Value::Null, vec![], vec![Symbol::from(rule.as_str())]));
                }
            }
            Ok((serde_json::Value::Null, vec![], vec![]))
        }
        async fn execute_action_step(
            &self,
            _: &str,
            _: &ActionStep,
            _: &HashMap<String, String>,
            _: &dyn DataAccess,
            _: &RuntimeContext,
        ) -> Result<bool, RuntimeError> {
            Ok(false)
        }
    }

    let engine = engine.with_plugins(vec![Box::new(MockFinancePlugin)]);
    let ctx = RuntimeContext::system();

    // 3. Create Accounts with System Tags
    engine
        .create(
            "Account",
            json!({
                "code": "101",
                "name": "Accounts Receivable Name", // Name is different from tag
                "system_tag": "accounts_receivable",
                "type": "Asset"
            }),
            &ctx,
        )
        .await
        .expect("Failed to create AR account");

    engine
        .create(
            "Account",
            json!({
                "code": "401",
                "name": "Sales Revenue Name", // Name is different from tag
                "system_tag": "sales_revenue",
                "type": "Revenue"
            }),
            &ctx,
        )
        .await
        .expect("Failed to create Revenue account");

    // 4. Create Invoice
    let invoice_data = json!({
        "invoice_number": "INV-001",
        "date": "2024-01-01",
        "total_amount": "1000.00"
    });
    let invoice_id = engine
        .create("Invoice", invoice_data, &ctx)
        .await
        .expect("Failed to create Invoice");

    // 5. Trigger Workflow (Post)
    let update_data = json!({
        "status": "Posted"
    });

    // This should trigger the posting rule using system_tags.
    engine
        .update("Invoice", &invoice_id, update_data, &ctx)
        .await
        .expect("Failed to post Invoice with system_tag lookup");

    // 6. Verify Journal Entry
    let journals = engine
        .list("JournalEntry", None, None, None, &ctx)
        .await
        .expect("Failed to list journals");
    assert_eq!(journals.len(), 1, "Should create exactly 1 journal entry");

    // 7. Verify Journal Lines (in DB)
    let lines = engine
        .list("JournalLine", None, None, None, &ctx)
        .await
        .expect("Failed to list journal lines");
    assert_eq!(lines.len(), 2, "Should create exactly 2 journal lines");
}
