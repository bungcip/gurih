use gurih_dsl::parser::parse;
use gurih_dsl::validator::Validator;

#[test]
fn test_valid_employee_status() {
    let src = r#"
    entity "Pegawai" {
        field:pk id
        field:string status
        field:date join_date
        field:boolean is_active
    }

    employee_status "Active" for="Pegawai" {
        can_transition_to "Inactive" {
            requires {
                min_years_of_service 1
            }
            effects {
                update "is_active" "false"
            }
        }
    }
    "#;

    let ast = parse(src, None).expect("Parse failed");
    Validator::new().validate(&ast).expect("Validation failed");
}

#[test]
fn test_invalid_entity() {
    let src = r#"
    employee_status "Active" for="UnknownEntity" {
        can_transition_to "Inactive"
    }
    "#;

    let ast = parse(src, None).expect("Parse failed");
    let err = Validator::new().validate(&ast).unwrap_err();
    assert!(err.to_string().contains("Entity 'UnknownEntity' not found"));
}

#[test]
fn test_invalid_precondition_field() {
    let src = r#"
    entity "Pegawai" {
        field:pk id
        field:date join_date
    }

    employee_status "Active" for="Pegawai" {
        can_transition_to "Inactive" {
            requires {
                // 'tmt_pns' does not exist
                // Note: Generic custom nodes are validated at runtime now.
                min_years_of_service 1 from="tmt_pns"
            }
        }
    }
    "#;

    let ast = parse(src, None).expect("Parse failed");
    // Validation should pass because 'min_years_of_service' is a generic Custom node
    // and the validator cannot verify its arguments (like 'from') without domain knowledge.
    Validator::new().validate(&ast).expect("Validation passed");
}

#[test]
fn test_invalid_effect_field() {
    let src = r#"
    entity "Pegawai" {
        field:pk id
        field:boolean is_active
    }

    employee_status "Active" for="Pegawai" {
        can_transition_to "Inactive" {
            effects {
                update "unknown_field" "false"
            }
        }
    }
    "#;

    let ast = parse(src, None).expect("Parse failed");
    let err = Validator::new().validate(&ast).unwrap_err();
    assert!(
        err.to_string()
            .contains("Effect target field 'unknown_field' not found")
    );
}

#[test]
fn test_employee_status_with_field() {
    let src = r#"
    entity "Pegawai" {
        field:pk id
        field:string status_pegawai
    }

    employee_status "Active" for="Pegawai" field="status_pegawai" {
        can_transition_to "Inactive"
    }
    "#;

    let ast = parse(src, None).expect("Parse failed");
    Validator::new().validate(&ast).expect("Validation failed");

    let status_def = &ast.employee_statuses[0];
    assert_eq!(status_def.field, Some("status_pegawai".to_string()));
}
