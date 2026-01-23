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
        | FieldType::Relation => val.is_string() || val.is_null(),
        FieldType::Integer => val.is_i64() || val.is_null(),
        FieldType::Float => val.is_f64() || val.is_null(),
        FieldType::Boolean => val.is_boolean() || val.is_null(),
    }
}
