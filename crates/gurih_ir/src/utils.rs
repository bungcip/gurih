use crate::DatabaseType;
use serde_json::Value;
use std::collections::HashMap;

pub fn to_title_case(s: &str) -> String {
    s.split(|c| c == '_' || c == '-')
        .filter(|s| !s.is_empty())
        .map(|word| {
            if word.eq_ignore_ascii_case("id") {
                return "ID".to_string();
            }
            let mut chars = word.chars();
            match chars.next() {
                None => String::new(),
                Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

pub fn capitalize(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        None => String::new(),
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
    }
}

pub fn to_snake_case(s: &str) -> String {
    let mut result = String::new();
    for (i, c) in s.char_indices() {
        if c.is_uppercase() {
            if i > 0 {
                result.push('_');
            }
            result.push(c.to_ascii_lowercase());
        } else {
            result.push(c);
        }
    }
    result
}

/// Resolves a parameter from the params map if the value matches "param(key)".
/// Supports both double ("key") and single ('key') quotes.
/// Otherwise returns the value as is.
pub fn resolve_param(val: &str, params: &HashMap<String, String>) -> String {
    if val.starts_with("param(") && val.ends_with(')') {
        let key = &val[6..val.len() - 1];
        let cleaned_key = key.trim_matches(|c| c == '"' || c == '\'');
        params.get(cleaned_key).cloned().unwrap_or(val.to_string())
    } else {
        val.to_string()
    }
}

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

/// Parses a JSON value into a f64 strictly.
/// Returns Ok(f64) if successful, otherwise Err with description.
pub fn parse_numeric_strict(v: &Value) -> Result<f64, String> {
    if let Some(f) = v.as_f64() {
        Ok(f)
    } else if let Some(s) = v.as_str() {
        s.parse().map_err(|_| format!("Invalid number format: {}", s))
    } else {
        Err(format!("Expected number, found {:?}", v))
    }
}

/// Helper to parse optional reference to Value
pub fn parse_numeric_opt(v: Option<&Value>) -> f64 {
    match v {
        Some(val) => parse_numeric(val),
        None => 0.0,
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

/// Returns a single database specific placeholder for the given index (1-based).
/// For Postgres: "$N"
/// For SQLite: "?"
pub fn get_db_placeholder(db_type: &DatabaseType, index: usize) -> String {
    if *db_type == DatabaseType::Postgres {
        format!("${}", index)
    } else {
        "?".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_to_title_case() {
        assert_eq!(to_title_case("first_name"), "First Name");
        assert_eq!(to_title_case("last_name"), "Last Name");
        assert_eq!(to_title_case("email"), "Email");
        assert_eq!(to_title_case("created_at"), "Created At");
        assert_eq!(to_title_case("sk_pns"), "Sk Pns");
        assert_eq!(to_title_case("my-field-name"), "My Field Name");
        assert_eq!(to_title_case("simple"), "Simple");
        assert_eq!(to_title_case(""), "");
        assert_eq!(to_title_case("____"), "");
        assert_eq!(to_title_case("weird__spacing"), "Weird Spacing");
        assert_eq!(to_title_case("id"), "ID");
        assert_eq!(to_title_case("user_id"), "User ID");
    }
}
