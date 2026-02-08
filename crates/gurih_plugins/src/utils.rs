use gurih_ir::DatabaseType;
use serde_json::Value;
use std::collections::HashMap;

/// Parses a JSON value into a f64.
/// Handles Numbers directly, and tries to parse Strings.
/// Returns 0.0 if parsing fails or value is null/other.
pub fn parse_numeric(v: &Value) -> f64 {
    if let Some(f) = v.as_f64() {
        f
    } else if let Some(s) = v.as_str() {
        s.parse().unwrap_or(0.0)
    } else {
        0.0
    }
}

/// Helper to parse optional reference to Value
pub fn parse_numeric_opt(v: Option<&Value>) -> f64 {
    match v {
        Some(val) => parse_numeric(val),
        None => 0.0,
    }
}

/// Resolves a parameter from the params map if the value matches "param(key)".
/// Otherwise returns the value as is.
pub fn resolve_param(val: &str, params: &HashMap<String, String>) -> String {
    if val.starts_with("param(") && val.ends_with(")") {
        let key = &val[6..val.len() - 1];
        let cleaned_key = key.trim_matches('"');
        params.get(cleaned_key).cloned().unwrap_or(val.to_string())
    } else {
        val.to_string()
    }
}

/// Returns the database specific placeholders for a range query (start, end).
/// For Postgres: ("$1", "$2")
/// For SQLite: ("?", "?")
pub fn get_db_range_placeholders(db_type: &DatabaseType) -> (&'static str, &'static str) {
    if *db_type == DatabaseType::Postgres {
        ("$1", "$2")
    } else {
        ("?", "?")
    }
}
