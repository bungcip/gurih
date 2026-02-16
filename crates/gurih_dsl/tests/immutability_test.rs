use gurih_dsl::compile;
use gurih_ir::{Ownership, RelationshipType, Symbol};

#[test]
fn test_composition_inference() {
    let src = r#"
    entity "Parent" {
        field:pk id
        has_many "children" "Child" type="composition"
    }

    entity "Child" {
        field:pk id
        belongs_to "Parent"
    }
    "#;

    let schema = compile(src, None).expect("Compilation failed");

    let child_entity = schema
        .entities
        .get(&Symbol::from("Child"))
        .expect("Child entity not found");

    let parent_rel = child_entity
        .relationships
        .iter()
        .find(|r| r.target_entity == Symbol::from("Parent"))
        .expect("Relationship to Parent not found");

    assert_eq!(parent_rel.rel_type, RelationshipType::BelongsTo, "Should be BelongsTo");
    assert_eq!(
        parent_rel.ownership,
        Ownership::Composition,
        "Ownership should be inferred as Composition"
    );
}
