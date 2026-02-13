use gurih_dsl::compiler::compile;
use gurih_ir::{FieldType, RelationshipType, Symbol};
use std::path::Path;

#[test]
fn test_finance_cost_center_support() {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let root_dir = Path::new(&manifest_dir).parent().unwrap().parent().unwrap();
    let finance_dir = root_dir.join("gurih-finance");
    let src_path = finance_dir.join("gurih.kdl");

    // Read the main file
    let src = std::fs::read_to_string(&src_path).expect("Failed to read gurih.kdl");

    // Compile with base path to resolve includes
    let schema = compile(&src, Some(&finance_dir)).expect("Compilation failed");

    // 1. Verify CostCenter Entity
    let cost_center_sym = Symbol::from("CostCenter");
    let cost_center = schema
        .entities
        .get(&cost_center_sym)
        .expect("CostCenter entity not found");

    // Check fields
    let code_field = cost_center
        .fields
        .iter()
        .find(|f| f.name == Symbol::from("code"))
        .unwrap();
    assert!(matches!(code_field.field_type, FieldType::String));

    let name_field = cost_center
        .fields
        .iter()
        .find(|f| f.name == Symbol::from("name"))
        .unwrap();
    assert!(matches!(name_field.field_type, FieldType::String));

    // 2. Verify JournalLine has CostCenter relationship
    let journal_line_sym = Symbol::from("JournalLine");
    let journal_line = schema
        .entities
        .get(&journal_line_sym)
        .expect("JournalLine entity not found");

    let cc_rel = journal_line
        .relationships
        .iter()
        .find(|r| r.target_entity == Symbol::from("CostCenter"));
    assert!(cc_rel.is_some(), "CostCenter relationship not found on JournalLine");
    let rel = cc_rel.unwrap();
    assert_eq!(rel.rel_type, RelationshipType::BelongsTo);

    // In KDL: belongs_to "cost_center" "CostCenter" -> name is "cost_center".
    assert_eq!(rel.name, Symbol::from("cost_center"));

    // 3. Verify IncomeStatementByCostCenterQuery
    let query_sym = Symbol::from("IncomeStatementByCostCenterQuery");
    let query = schema
        .queries
        .get(&query_sym)
        .expect("IncomeStatementByCostCenterQuery not found");

    // Check params
    let param_sym = Symbol::from("cost_center_id");
    assert!(query.params.contains(&param_sym), "cost_center_id parameter missing");

    // 4. Verify Route exists
    // The key in ir_routes is the group path if it's a group
    let finance_group = schema.routes.get("/finance").expect("Finance group not found");

    // Check children
    let report_route = finance_group
        .children
        .iter()
        .find(|r| r.path == "/reports/income-statement-by-cost-center");
    assert!(report_route.is_some(), "Report route not found in /finance group");
}
