use crate::storage::Storage;
use crate::workflow::WorkflowEngine;
use crate::context::RuntimeContext;
use gurih_ir::{Schema, FieldType};
use serde_json::Value;
use std::sync::Arc;

pub struct DataEngine {
    schema: Arc<Schema>,
    storage: Arc<dyn Storage>,
    workflow: WorkflowEngine,
}

impl DataEngine {
    pub fn new(schema: Arc<Schema>, storage: Arc<dyn Storage>) -> Self {
        Self { 
            schema, 
            storage,
            workflow: WorkflowEngine::new(),
        }
    }

    pub async fn create(&self, entity_name: &str, mut data: Value, ctx: &RuntimeContext) -> Result<String, String> {
        // TODO: Validate create permission for entity
        
        let entity_schema = self.schema.entities.get(entity_name)
            .ok_or_else(|| format!("Entity '{}' not defined", entity_name))?;

        // Workflow: Set initial state if applicable
        if let Some(initial_state) = self.workflow.get_initial_state(&self.schema, entity_name) {
             if let Some(obj) = data.as_object_mut() {
                 if !obj.contains_key("state") {
                     obj.insert("state".to_string(), Value::String(initial_state));
                 }
             }
        }

        // Validation
        if let Some(obj) = data.as_object() {
            for field in &entity_schema.fields {
                if field.required && !obj.contains_key(&field.name) {
                    return Err(format!("Missing required field: {}", field.name));
                }
                
                if let Some(val) = obj.get(&field.name) {
                    if !validate_type(val, &field.field_type) {
                        return Err(format!("Invalid type for field: {}", field.name));
                    }
                }
            }
        } else {
            return Err("Data must be an object".to_string());
        }

        self.storage.insert(entity_name, data).await
    }
    
    pub async fn read(&self, entity_name: &str, id: &str) -> Result<Option<Value>, String> {
        if !self.schema.entities.contains_key(entity_name) {
             return Err(format!("Entity '{}' not defined", entity_name));
        }
        self.storage.get(entity_name, id).await
    }
    
    pub async fn update(&self, entity_name: &str, id: &str, data: Value, ctx: &RuntimeContext) -> Result<(), String> {
         let entity_schema = self.schema.entities.get(entity_name)
            .ok_or_else(|| format!("Entity '{}' not defined", entity_name))?;

         // Workflow Transition Check
         if let Some(new_state) = data.get("state").and_then(|v| v.as_str()) {
             // We only check if there IS a workflow for this entity
             if self.schema.workflows.values().any(|w| w.entity == entity_name) {
                 let current_record = self.storage.get(entity_name, id).await?
                     .ok_or("Record not found")?;
                 
                 let current_state = current_record.get("state")
                    .and_then(|v| v.as_str())
                    .unwrap_or(""); // Assume empty state if missing
                 
                 // Validate transition logic
                 self.workflow.validate_transition(&self.schema, entity_name, current_state, new_state)?;
                 
                 // Validate permissions for transition
                 if let Some(perm) = self.workflow.get_transition_permission(&self.schema, entity_name, current_state, new_state) {
                     if !ctx.has_permission(&perm) {
                         return Err(format!("Missing permission '{}' for transition", perm));
                     }
                 }
             }
         }
            
         // Validation
         if let Some(obj) = data.as_object() {
            for field in &entity_schema.fields {
                 if let Some(val) = obj.get(&field.name) {
                    if !validate_type(val, &field.field_type) {
                        return Err(format!("Invalid type for field: {}", field.name));
                    }
                }
            }
        }
        
        self.storage.update(entity_name, id, data).await
    }
    
    pub async fn delete(&self, entity_name: &str, id: &str) -> Result<(), String> {
         if !self.schema.entities.contains_key(entity_name) {
             return Err(format!("Entity '{}' not defined", entity_name));
        }
        self.storage.delete(entity_name, id).await
    }
    
    pub async fn list(&self, entity_name: &str) -> Result<Vec<Value>, String> {
         if !self.schema.entities.contains_key(entity_name) {
             return Err(format!("Entity '{}' not defined", entity_name));
        }
        self.storage.list(entity_name).await
    }
}

fn validate_type(val: &Value, field_type: &FieldType) -> bool {
    match field_type {
        FieldType::String | FieldType::Text | FieldType::Enum(_) | FieldType::Date | FieldType::DateTime | FieldType::Relation => val.is_string(),
        FieldType::Integer => val.is_i64(),
        FieldType::Float => val.is_f64(),
        FieldType::Boolean => val.is_boolean(),
    }
}
