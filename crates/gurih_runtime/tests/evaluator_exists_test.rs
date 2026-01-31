use gurih_ir::{Expression, Symbol};
use gurih_runtime::datastore::{DataStore, MemoryDataStore};
use gurih_runtime::evaluator::evaluate;
use serde_json::json;
use std::sync::Arc;

#[tokio::test]
async fn test_exists_function() {
    let datastore: Arc<dyn DataStore> = Arc::new(MemoryDataStore::new());

    // 1. Seed Data
    // Note: evaluator::exists uses to_snake_case("JournalLine") -> "journal_line"
    // So we seed "journal_line"
    datastore
        .insert(
            "journal_line",
            json!({
                "id": "1",
                "account": "acc1",
                "amount": 100.0
            }),
        )
        .await
        .unwrap();

    datastore
        .insert(
            "journal_line",
            json!({
                "id": "2",
                "account": "acc1",
                "amount": 200
            }),
        )
        .await
        .unwrap();

    datastore
        .insert(
            "journal_line",
            json!({
                "id": "3",
                "account": "acc2",
                "amount": 50
            }),
        )
        .await
        .unwrap();

    // 2. Test exists("JournalLine", "account", "acc1") -> True
    let expr = Expression::FunctionCall {
        name: Symbol::from("exists"),
        args: vec![
            Expression::StringLiteral("JournalLine".to_string()),
            Expression::StringLiteral("account".to_string()),
            Expression::StringLiteral("acc1".to_string()),
        ],
    };
    let ctx = json!({});
    let res = evaluate(&expr, &ctx, None, Some(&datastore)).await.unwrap();
    assert_eq!(res, json!(true));

    // 3. Test exists("JournalLine", "account", "acc3") -> False
    let expr = Expression::FunctionCall {
        name: Symbol::from("exists"),
        args: vec![
            Expression::StringLiteral("JournalLine".to_string()),
            Expression::StringLiteral("account".to_string()),
            Expression::StringLiteral("acc3".to_string()),
        ],
    };
    let res = evaluate(&expr, &ctx, None, Some(&datastore)).await.unwrap();
    assert_eq!(res, json!(false));

    // 4. Test exists("JournalLine", "account", "acc1", "amount", 100) -> True
    let expr = Expression::FunctionCall {
        name: Symbol::from("exists"),
        args: vec![
            Expression::StringLiteral("JournalLine".to_string()),
            Expression::StringLiteral("account".to_string()),
            Expression::StringLiteral("acc1".to_string()),
            Expression::StringLiteral("amount".to_string()),
            Expression::Literal(100.0),
        ],
    };
    let res = evaluate(&expr, &ctx, None, Some(&datastore)).await.unwrap();
    assert_eq!(res, json!(true));

    // 5. Test exists("JournalLine", "account", "acc1", "amount", 999) -> False
    let expr = Expression::FunctionCall {
        name: Symbol::from("exists"),
        args: vec![
            Expression::StringLiteral("JournalLine".to_string()),
            Expression::StringLiteral("account".to_string()),
            Expression::StringLiteral("acc1".to_string()),
            Expression::StringLiteral("amount".to_string()),
            Expression::Literal(999.0),
        ],
    };
    let res = evaluate(&expr, &ctx, None, Some(&datastore)).await.unwrap();
    assert_eq!(res, json!(false));
}
