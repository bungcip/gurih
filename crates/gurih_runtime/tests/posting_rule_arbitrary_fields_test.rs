use gurih_dsl::compiler::compile;
use gurih_runtime::context::RuntimeContext;
use gurih_runtime::data::DataEngine;
use gurih_runtime::datastore::{DataStore, MemoryDataStore};
use serde_json::json;
use std::sync::Arc;

#[tokio::test]
async fn test_posting_rule_arbitrary_fields() {
    let kdl = r#"
    entity "Account" {
        field:pk id
        field:string "code" unique=#true
        field:string "name" unique=#true
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

        // Arbitrary fields we want to set
        field:string "party_type"
        field:string "party_id"
    }

    entity "Invoice" {
        field:pk id
        field:string "invoice_number"
        field:date "date"
        field:money "total_amount"
        field:string "customer_id"
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
            account "101"
            debit "doc.total_amount"

            // This syntax is currently unsupported and should be parsed
            set "party_type" "'Customer'"
            set "party_id" "doc.customer_id"
        }

        entry {
            account "401"
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

    // 3. Create Accounts
    engine
        .create(
            "Account",
            json!({
                "code": "101",
                "name": "Accounts Receivable",
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
                "name": "Sales Revenue",
                "type": "Revenue"
            }),
            &ctx,
        )
        .await
        .expect("Failed to create Revenue account");

    // 4. Create Invoice
    let customer_id = "CUST-001";
    let invoice_data = json!({
        "invoice_number": "INV-001",
        "date": "2024-01-01",
        "total_amount": "1000.00",
        "customer_id": customer_id
    });
    let invoice_id = engine
        .create("Invoice", invoice_data, &ctx)
        .await
        .expect("Failed to create Invoice");

    // 5. Trigger Workflow (Post)
    let update_data = json!({
        "status": "Posted"
    });

    engine
        .update("Invoice", &invoice_id, update_data, &ctx)
        .await
        .expect("Failed to post Invoice");

    // 6. Verify Journal Lines
    let lines = engine
        .list("JournalLine", None, None, None, &ctx)
        .await
        .expect("Failed to list journal lines");

    // Find the debit line (AR)
    let ar_line = lines.iter().find(|l| {
        let debit = l.get("debit").and_then(|v| v.as_str()).unwrap_or("0");
        debit == "1000.00" || debit == "1000.0"
    }).expect("AR line not found");

    // Assert arbitrary fields are set
    let party_type = ar_line.get("party_type").and_then(|v| v.as_str());
    let party_id = ar_line.get("party_id").and_then(|v| v.as_str());

    assert_eq!(party_type, Some("Customer"), "party_type should be set");
    assert_eq!(party_id, Some(customer_id), "party_id should be set");
}
