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
    Validator::new(src).validate(&ast).expect("Validation failed");
}

#[test]
fn test_invalid_entity() {
    let src = r#"
    employee_status "Active" for="UnknownEntity" {
        can_transition_to "Inactive"
    }
    "#;

    let ast = parse(src, None).expect("Parse failed");
    let err = Validator::new(src).validate(&ast).unwrap_err();
    assert!(format!("{}", err).contains("Employee status entity 'UnknownEntity' not found"));
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
                min_years_of_service 1 from="tmt_pns"
            }
        }
    }
    "#;

    let ast = parse(src, None).expect("Parse failed");
    let err = Validator::new(src).validate(&ast).unwrap_err();
    assert!(format!("{}", err).contains("Field 'tmt_pns' not found in entity"));
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
    let err = Validator::new(src).validate(&ast).unwrap_err();
    assert!(format!("{}", err).contains("Effect target field 'unknown_field' not found"));
}
