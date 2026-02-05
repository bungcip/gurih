use gurih_dsl::compiler::compile;
use gurih_ir::TransitionEffect;

#[test]
fn test_compile_posting_rule() {
    let src = r#"
    entity "Invoice" {
        pk "id"
        field "number" type="String"
        field "date" type="Date"
        field "total" type="Money"
        field "status" type="String"
    }

    posting_rule "InvoicePosting" for="Invoice" {
        description "\"Inv \" + doc.number"
        date "doc.date"

        entry {
            account "Accounts Receivable"
            debit "doc.total"
        }

        entry {
            account "Sales Revenue"
            credit "doc.total"
        }
    }

    workflow "InvoiceFlow" for="Invoice" field="status" {
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
    "#;

    let schema = compile(src, None).expect("Should compile");

    // Check Posting Rule
    assert!(
        schema
            .posting_rules
            .contains_key(&gurih_ir::Symbol::from("InvoicePosting"))
    );
    let rule = schema
        .posting_rules
        .get(&gurih_ir::Symbol::from("InvoicePosting"))
        .unwrap();
    assert_eq!(rule.lines.len(), 2);

    // Check Workflow Effect
    let wf = schema.workflows.get(&gurih_ir::Symbol::from("InvoiceFlow")).unwrap();
    let transition = &wf.transitions[0];
    let effect = &transition.effects[0];

    match effect {
        TransitionEffect::Custom { name, args, .. } if name.as_str() == "post_journal" => {
            if let gurih_ir::Expression::StringLiteral(rule_name) = &args[0] {
                assert_eq!(rule_name, "InvoicePosting");
            } else {
                panic!("Expected StringLiteral");
            }
        }
        _ => panic!("Expected PostJournal effect"),
    }
}
