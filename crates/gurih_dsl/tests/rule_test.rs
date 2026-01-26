use gurih_dsl::compiler::compile;
use gurih_ir::{BinaryOperator, Expression, Symbol};

#[test]
fn test_compile_rule() {
    let src = r#"
        rule "DebitEqualsCredit" {
            on "JournalEntry.post"
            assert "lines.debit == lines.credit"
            message "Debit and Credit must be equal"
        }
    "#;

    let schema = compile(src, None).expect("Should compile");
    assert!(schema.rules.contains_key(&Symbol::from("DebitEqualsCredit")));
    let rule = schema.rules.get(&Symbol::from("DebitEqualsCredit")).unwrap();
    assert_eq!(rule.on_event.as_str(), "JournalEntry.post");
    assert_eq!(rule.message, "Debit and Credit must be equal");

    match &rule.assertion {
        Expression::BinaryOp { left, op, right } => {
            assert!(matches!(op, BinaryOperator::Eq));
            // Check left is lines.debit
            if let Expression::Field(s) = &**left {
                assert_eq!(s.as_str(), "lines.debit");
            } else {
                panic!("Left should be field");
            }
            if let Expression::Field(s) = &**right {
                assert_eq!(s.as_str(), "lines.credit");
            } else {
                panic!("Right should be field");
            }
        }
        _ => panic!("Expected BinaryOp"),
    }
}

#[test]
fn test_compile_rule_boolean() {
    let src = r#"
        rule "CheckPeriod" {
            on "Post"
            assert "period.closed == false"
            message "Period is closed"
        }
    "#;
    let schema = compile(src, None).expect("Should compile");
    let rule = schema.rules.get(&Symbol::from("CheckPeriod")).unwrap();
    match &rule.assertion {
        Expression::BinaryOp { right, .. } => {
            if let Expression::BoolLiteral(b) = &**right {
                assert!(!(*b));
            } else {
                panic!("Right should be bool literal");
            }
        }
        _ => panic!("Expected BinaryOp"),
    }
}

#[test]
fn test_compile_rule_complex_logic() {
    let src = r#"
        rule "Complex" {
            on "Save"
            assert "amount > 0 && (status == \"draft\" || role == \"admin\")"
            message "Invalid state"
        }
    "#;
    let schema = compile(src, None).expect("Should compile");
    let rule = schema.rules.get(&Symbol::from("Complex")).unwrap();
    // Just verifying it compiles and produces AST
    println!("{:?}", rule.assertion);
}

#[test]
#[should_panic(expected = "ValidationError")]
fn test_compile_rule_type_error() {
    let src = r#"
        rule "BadType" {
            on "Save"
            assert "1 + true"
            message "Invalid math"
        }
    "#;
    compile(src, None).unwrap();
}

#[test]
#[should_panic(expected = "ValidationError")]
fn test_compile_rule_non_bool_assert() {
    let src = r#"
        rule "BadReturn" {
            on "Save"
            assert "1 + 1"
            message "Returns number"
        }
    "#;
    compile(src, None).unwrap();
}
