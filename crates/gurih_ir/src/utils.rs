use std::collections::HashMap;

pub fn to_title_case(s: &str) -> String {
    s.split('_')
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
