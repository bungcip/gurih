use gurih_dsl::compiler::compile;
use gurih_plugins::finance::FinancePlugin;
use gurih_runtime::context::RuntimeContext;
use gurih_runtime::data::DataEngine;
use gurih_runtime::datastore::DataStore;
use gurih_runtime::store::MemoryDataStore;
use serde_json::json;
use std::sync::Arc;

#[tokio::test]
async fn test_posting_rule_execution() {
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
            account "Accounts Receivable"
            debit "doc.total_amount"
        }

        entry {
            account "Sales Revenue"
            credit "doc.total_amount"
        }
    }
    "#;

    // 1. Compile Schema
    let schema = compile(kdl, None).expect("Failed to compile schema");
    let schema_arc = Arc::new(schema);

    // 2. Setup Runtime
    let datastore: Arc<dyn DataStore> = Arc::new(MemoryDataStore::new());
    let engine = DataEngine::new(schema_arc.clone(), datastore.clone()).with_plugins(vec![Box::new(FinancePlugin)]);
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

    // This should trigger the posting rule
    engine
        .update("Invoice", &invoice_id, update_data, &ctx)
        .await
        .expect("Failed to post Invoice");

    // 6. Verify Journal Entry
    let journals = engine
        .list("JournalEntry", None, None, None, &ctx)
        .await
        .expect("Failed to list journals");
    assert_eq!(journals.len(), 1, "Should create exactly 1 journal entry");

    let journal = journals[0].clone();
    assert_eq!(journal.get("description").unwrap(), "Invoice #INV-001");
    assert_eq!(journal.get("date").unwrap(), "2024-01-01");

    // 7. Verify Journal Lines (in DB)
    let lines = engine
        .list("JournalLine", None, None, None, &ctx)
        .await
        .expect("Failed to list journal lines");
    assert_eq!(lines.len(), 2, "Should create exactly 2 journal lines");

    // Check line details
    let debit_line = lines
        .iter()
        .find(|l| {
            let d_str = l.get("debit").and_then(|v| v.as_str()).unwrap_or("0");
            let d = d_str.parse::<f64>().unwrap_or(0.0);
            d > 0.0
        })
        .expect("Should have a debit line");

    // We expect "1000.00" because it comes from the Invoice exactly as string
    assert_eq!(debit_line.get("debit").unwrap(), "1000.00");

    let credit_line = lines
        .iter()
        .find(|l| {
            let c_str = l.get("credit").and_then(|v| v.as_str()).unwrap_or("0");
            let c = c_str.parse::<f64>().unwrap_or(0.0);
            c > 0.0
        })
        .expect("Should have a credit line");

    assert_eq!(credit_line.get("credit").unwrap(), "1000.00");

    // Verify linkage
    let journal_id = journal.get("id").unwrap().as_str().unwrap();
    assert_eq!(debit_line.get("journal_entry").unwrap().as_str().unwrap(), journal_id);
    assert_eq!(credit_line.get("journal_entry").unwrap().as_str().unwrap(), journal_id);
}
