use gurih_dsl::compiler::compile;
use std::fs;

#[test]
fn test_compile_golden_master() {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let file_path = std::path::Path::new(&manifest_dir).join("../../gurih-hr/gurih.kdl");

    let src = fs::read_to_string(file_path).expect("Failed to read gurih.kdl");
    let schema = compile(&src).expect("Failed to compile");

    println!("Compiled Schema: {}", schema.name);
    assert_eq!(schema.name, "GurihHR");

    // Check Modules
    assert!(schema.modules.contains_key("Organization"));
    assert!(schema.modules.contains_key("Personnel"));
    assert!(schema.modules.contains_key("Leave"));

    // Check Entities (Validation that they are registered in the global map)
    assert!(schema.entities.contains_key("Employee"));
    let emp = schema.entities.get("Employee").unwrap();

    // Check Fields & Serials
    assert_eq!(emp.fields.len(), 7, "Employee should have 7 fields");

    // Check Serial usage
    let code_field = emp
        .fields
        .iter()
        .find(|f| f.name == "employee_id")
        .expect("Should have employee_id field");
    assert_eq!(code_field.serial.as_deref(), Some("EmpCode"));

    // Check Relationships
    // belongs_to Department, Position
    // has_many leave_requests
    assert_eq!(emp.relationships.len(), 3);

    // Check Options
    assert_eq!(
        emp.options.get("track_changes").map(|s| s.as_str()),
        Some("true")
    );

    // Check Serials Definition
    assert!(schema.serials.contains_key("EmpCode"));

    // Check Layouts
    assert!(schema.layouts.contains_key("MainLayout"));
    let layout = schema.layouts.get("MainLayout").unwrap();
    assert!(layout.header_enabled);
    assert!(layout.props.contains_key("search_bar"));

    // Check Menus
    assert!(schema.menus.contains_key("MainMenu"));

    // Check Dashboards
    assert!(schema.dashboards.contains_key("HRDashboard"));

    // Check Pages
    assert!(schema.pages.contains_key("EmployeeList"));

    // Check Routes
    assert!(schema.routes.contains_key("/"));
    assert!(schema.routes.contains_key("/employees"));
}

#[test]
fn test_compile_widget_icon() {
    let src = r#"
    name "TestApp"
    dashboard "MainDash" {
        widget "Stats" type="Stat" {
            label "Total Users"
            value "100"
            icon "lucide:users"
        }
    }
    "#;

    let schema = compile(src).expect("Failed to compile");
    let dash = schema
        .dashboards
        .get("MainDash")
        .expect("Dashboard not found");
    let widget = dash.widgets.first().expect("Widget not found");

    assert_eq!(widget.name, "Stats");
    assert_eq!(widget.icon, Some("lucide:users".to_string()));
}
