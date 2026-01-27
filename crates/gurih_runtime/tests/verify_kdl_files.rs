use gurih_dsl::compile;
use std::path::Path;

#[test]
fn test_verify_gurih_siasn() {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    // Repo root is ../../ from crates/gurih_runtime
    let root = Path::new(&manifest_dir).parent().unwrap().parent().unwrap();
    let siasn_path = root.join("gurih-siasn");
    let app_kdl = siasn_path.join("app.kdl");

    let content = std::fs::read_to_string(&app_kdl).expect("Failed to read app.kdl");

    let schema = compile(&content, Some(&siasn_path));
    if let Err(e) = &schema {
        println!("Error compiling SIASN: {:?}", e);
    }
    assert!(schema.is_ok(), "GurihSIASN schema failed to compile");
}

#[test]
fn test_verify_gurih_finance() {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let root = Path::new(&manifest_dir).parent().unwrap().parent().unwrap();
    let finance_path = root.join("gurih-finance");
    let gurih_kdl = finance_path.join("gurih.kdl");

    let content = std::fs::read_to_string(&gurih_kdl).expect("Failed to read gurih.kdl");

    let schema = compile(&content, Some(&finance_path));
    if let Err(e) = &schema {
        println!("Error compiling Finance: {:?}", e);
    }
    assert!(schema.is_ok(), "GurihFinance schema failed to compile");
}
