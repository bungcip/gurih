use gurih_dsl::parser::parse;
use gurih_ir::ActionStepType;

#[test]
fn test_parse_entity_update_step() {
    let input = r#"
        action "UpdateUser" {
            step "entity:update" target="User" id="param(id)" role="Admin"
        }
    "#;

    let ast = parse(input, None).expect("Should parse action with entity:update");
    assert_eq!(ast.actions.len(), 1);

    let action = &ast.actions[0];
    assert_eq!(action.steps.len(), 1);

    let step = &action.steps[0];
    assert_eq!(step.step_type, ActionStepType::EntityUpdate);
    assert_eq!(step.target, "User");
    assert_eq!(step.args.get("id"), Some(&"param(id)".to_string()));
    assert_eq!(step.args.get("role"), Some(&"Admin".to_string()));
}

#[test]
fn test_parse_entity_delete_step() {
    let input = r#"
        action "DeleteUser" {
            step "entity:delete" target="User" id="param(id)"
        }
    "#;

    let ast = parse(input, None).expect("Should parse action with entity:delete");
    assert_eq!(ast.actions.len(), 1);

    let step = &ast.actions[0].steps[0];
    assert_eq!(step.step_type, ActionStepType::EntityDelete);
}

#[test]
fn test_parse_custom_step() {
    let input = r#"
        action "CustomAction" {
            step "email:send" target="User" template="welcome"
        }
    "#;

    let ast = parse(input, None).expect("Should parse action with custom step");
    assert_eq!(ast.actions.len(), 1);

    let step = &ast.actions[0].steps[0];
    match &step.step_type {
        ActionStepType::Custom(name) => assert_eq!(name, "email:send"),
        _ => panic!("Expected Custom step type"),
    }
}
