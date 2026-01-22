use crate::auth::hash_password;
use crate::context::RuntimeContext;
use crate::datastore::DataStore;
use crate::query_engine::{QueryEngine, QueryPlan};
use crate::workflow::WorkflowEngine;
use gurih_ir::{FieldType, Schema, Symbol};
use serde_json::Value;
use std::sync::Arc;

pub struct DataEngine {
    schema: Arc<Schema>,
    datastore: Arc<dyn DataStore>,
    workflow: WorkflowEngine,
}

impl DataEngine {
    pub fn new(schema: Arc<Schema>, datastore: Arc<dyn DataStore>) -> Self {
        Self {
            schema,
            datastore,
            workflow: WorkflowEngine::new(),
        }
    }

    pub fn get_schema(&self) -> &Schema {
        &self.schema
    }

    pub fn datastore(&self) -> &Arc<dyn DataStore> {
        &self.datastore
    }

    pub async fn create(&self, entity_name: &str, mut data: Value, _ctx: &RuntimeContext) -> Result<String, String> {
        // TODO: Validate create permission for entity

        let entity_schema = self
            .schema
            .entities
            .get(&Symbol::from(entity_name))
            .ok_or_else(|| format!("Entity '{}' not defined", entity_name))?;

        // Workflow: Set initial state if applicable
        if let Some(initial_state) = self.workflow.get_initial_state(&self.schema, entity_name)
            && let Some(obj) = data.as_object_mut()
            && !obj.contains_key("state")
        {
            obj.insert("state".to_string(), Value::String(initial_state));
        }

        // Validation & Transformation (Hashing)
        if let Some(obj) = data.as_object_mut() {
            for field in &entity_schema.fields {
                if field.required && !obj.contains_key(&field.name.to_string()) {
                    return Err(format!("Missing required field: {}", field.name));
                }

                if let Some(val) = obj.get_mut(field.name.as_str()) {
                    if !validate_type(val, &field.field_type) {
                        return Err(format!("Invalid type for field: {}", field.name));
                    }
                    // Hash password if applicable
                    if field.field_type == FieldType::Password {
                        if let Value::String(pass) = val {
                            *val = Value::String(hash_password(pass));
                        }
                    }
                }
            }
        } else {
            return Err("Data must be an object".to_string());
        }

        self.datastore.insert(entity_name, data).await
    }

    pub async fn read(&self, entity_name: &str, id: &str) -> Result<Option<Arc<Value>>, String> {
        if !self.schema.entities.contains_key(&Symbol::from(entity_name)) {
            return Err(format!("Entity '{}' not defined", entity_name));
        }
        self.datastore.get(entity_name, id).await
    }

    pub async fn update(&self, entity_name: &str, id: &str, data: Value, ctx: &RuntimeContext) -> Result<(), String> {
        let entity_schema = self
            .schema
            .entities
            .get(&Symbol::from(entity_name))
            .ok_or_else(|| format!("Entity '{}' not defined", entity_name))?;

        // Workflow Transition Check
        if let Some(new_state) = data.get("state").and_then(|v| v.as_str()) {
            // We only check if there IS a workflow for this entity
            if self
                .schema
                .workflows
                .values()
                .any(|w| w.entity == Symbol::from(entity_name))
            {
                let current_record = self.datastore.get(entity_name, id).await?.ok_or("Record not found")?;

                let current_state = current_record.get("state").and_then(|v| v.as_str()).unwrap_or(""); // Assume empty state if missing

                // Validate transition logic
                self.workflow
                    .validate_transition(&self.schema, entity_name, current_state, new_state)?;

                // Validate permissions for transition
                if let Some(perm) =
                    self.workflow
                        .get_transition_permission(&self.schema, entity_name, current_state, new_state)
                    && !ctx.has_permission(&perm)
                {
                    return Err(format!("Missing permission '{}' for transition", perm));
                }
            }
        }

        // Validation & Transformation (Hashing)
        let mut data = data;
        if let Some(obj) = data.as_object_mut() {
            for field in &entity_schema.fields {
                if let Some(val) = obj.get_mut(field.name.as_str()) {
                    if !validate_type(val, &field.field_type) {
                        return Err(format!("Invalid type for field: {}", field.name));
                    }
                    // Hash password if applicable
                    if field.field_type == FieldType::Password {
                        if let Value::String(pass) = val {
                            *val = Value::String(hash_password(pass));
                        }
                    }
                }
            }
        }

        self.datastore.update(entity_name, id, data).await
    }

    pub async fn delete(&self, entity_name: &str, id: &str) -> Result<(), String> {
        if !self.schema.entities.contains_key(&Symbol::from(entity_name)) {
            return Err(format!("Entity '{}' not defined", entity_name));
        }
        self.datastore.delete(entity_name, id).await
    }

    pub async fn list(
        &self,
        entity: &str,
        limit: Option<usize>,
        offset: Option<usize>,
    ) -> Result<Vec<Arc<Value>>, String> {
        if self.schema.queries.contains_key(&Symbol::from(entity)) {
            let strategy = QueryEngine::plan(&self.schema, entity)?;
            if let Some(QueryPlan::ExecuteSql { mut sql }) = strategy.plans.first().cloned() {
                if let Some(l) = limit {
                    sql.push_str(&format!(" LIMIT {}", l));
                }
                if let Some(o) = offset {
                    sql.push_str(&format!(" OFFSET {}", o));
                }
                return self.datastore.query(&sql).await;
            }
            return Err("Query engine failed to produce SQL plan".to_string());
        }

        if !self.schema.entities.contains_key(&Symbol::from(entity)) {
            return Err(format!("Entity or Query '{}' not defined", entity));
        }
        self.datastore.list(entity, limit, offset).await
    }
}

fn validate_type(val: &Value, field_type: &FieldType) -> bool {
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
        | FieldType::Relation => val.is_string() || val.is_null(),
        FieldType::Integer => val.is_i64() || val.is_null(),
        FieldType::Float => val.is_f64() || val.is_null(),
        FieldType::Boolean => val.is_boolean() || val.is_null(),
    }
}
