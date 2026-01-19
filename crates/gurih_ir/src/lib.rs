use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Schema {
    pub version: String,
    pub entities: HashMap<String, EntitySchema>,
    pub workflows: HashMap<String, WorkflowSchema>,
    pub forms: HashMap<String, FormSchema>,
    pub permissions: HashMap<String, PermissionSchema>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntitySchema {
    pub name: String,
    pub fields: Vec<FieldSchema>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldSchema {
    pub name: String,
    pub field_type: FieldType,
    pub required: bool,
    pub unique: bool,
    pub default: Option<String>,
    pub references: Option<String>, // Entity name for relations
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum FieldType {
    String,
    Text, // Long string
    Integer,
    Float,
    Boolean,
    Date,
    DateTime,
    Enum(Vec<String>),
    Relation, // One-to-One or Many-to-One usually
    // JSON,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowSchema {
    pub name: String,
    pub entity: String,
    pub initial_state: String,
    pub states: Vec<String>,
    pub transitions: Vec<Transition>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transition {
    pub name: String,
    pub from: String,
    pub to: String,
    pub required_permission: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormSchema {
    pub name: String,
    pub entity: String,
    pub sections: Vec<FormSection>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormSection {
    pub title: String,
    pub fields: Vec<String>, // Field names
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionSchema {
    pub name: String,
    pub rules: Vec<String>, 
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_serialization() {
        let field = FieldSchema {
            name: "title".to_string(),
            field_type: FieldType::String,
            required: true,
            unique: false,
            default: None,
            references: None,
        };

        let entity = EntitySchema {
            name: "Book".to_string(),
            fields: vec![field],
        };

        let mut entities = HashMap::new();
        entities.insert("Book".to_string(), entity);

        let schema = Schema {
            version: "1.0".to_string(),
            entities,
            workflows: HashMap::new(),
            forms: HashMap::new(),
            permissions: HashMap::new(),
        };

        let json = serde_json::to_string_pretty(&schema).unwrap();
        println!("{}", json);

        let deserialized: Schema = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.version, "1.0");
        assert!(deserialized.entities.contains_key("Book"));
    }
}
