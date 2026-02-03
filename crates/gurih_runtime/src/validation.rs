use gurih_ir::FieldType;
use serde_json::Value;

pub fn validate_type(val: &Value, field_type: &FieldType) -> bool {
    match field_type {
        FieldType::Pk
        | FieldType::Serial
        | FieldType::Sku
        | FieldType::Name
        | FieldType::Title
        | FieldType::Description
        | FieldType::Avatar
        | FieldType::Money
        | FieldType::Email
        | FieldType::Phone
        | FieldType::Address
        | FieldType::Password
        | FieldType::Enum(_)
        | FieldType::Date
        | FieldType::Timestamp
        | FieldType::String
        | FieldType::Text
        | FieldType::Image
        | FieldType::File
        | FieldType::Code
        | FieldType::Custom(_)
        | FieldType::Uuid
        | FieldType::Relation => val.is_string() || val.is_null(),
        FieldType::Integer => val.is_i64() || val.is_null(),
        FieldType::Float => val.is_f64() || val.is_null(),
        FieldType::Boolean => val.is_boolean() || val.is_null(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_validate_type_coverage() {
        // Integer
        assert!(validate_type(&json!(123), &FieldType::Integer));
        assert!(validate_type(&json!(null), &FieldType::Integer));
        assert!(!validate_type(&json!("123"), &FieldType::Integer));

        // Float
        assert!(validate_type(&json!(12.3), &FieldType::Float));
        assert!(validate_type(&json!(null), &FieldType::Float));
        assert!(!validate_type(&json!("12.3"), &FieldType::Float));

        // Boolean
        assert!(validate_type(&json!(true), &FieldType::Boolean));
        assert!(validate_type(&json!(null), &FieldType::Boolean));
        assert!(!validate_type(&json!("true"), &FieldType::Boolean));

        // String group
        assert!(validate_type(&json!("hello"), &FieldType::String));
        assert!(validate_type(&json!(null), &FieldType::String));
        assert!(!validate_type(&json!(123), &FieldType::String));

        // Ensure other types in the string group are covered by implication
        assert!(validate_type(&json!("code"), &FieldType::Code));
    }
}
