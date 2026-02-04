use serde_json::Value;

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
