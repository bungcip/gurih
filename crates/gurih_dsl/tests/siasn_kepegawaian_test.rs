use gurih_dsl::parser::parse;
use std::path::Path;

#[test]
fn test_siasn_kepegawaian_parsing() {
    let path = Path::new("../../gurih-siasn/kepegawaian.kdl");
    let content = std::fs::read_to_string(path).expect("Failed to read kepegawaian.kdl");
    // We expect the parse to succeed
    let ast = parse(&content, None).expect("Failed to parse kepegawaian.kdl");

    // Check if RiwayatHukumanDisiplin entity exists in the Kepegawaian module
    // Note: 'modules' might be a top-level field in AST

    let entity = ast
        .modules
        .iter()
        .flat_map(|m| &m.entities)
        .find(|e| e.name == "RiwayatHukumanDisiplin")
        .expect("RiwayatHukumanDisiplin entity not found");

    // Check fields
    assert!(
        entity.fields.iter().any(|f| f.name == "tingkat_hukuman"),
        "tingkat_hukuman field missing"
    );
    assert!(
        entity.fields.iter().any(|f| f.name == "jenis_hukuman"),
        "jenis_hukuman field missing"
    );
    assert!(
        entity.fields.iter().any(|f| f.name == "nomor_sk"),
        "nomor_sk field missing"
    );
}
