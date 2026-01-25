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
        if let Some(wf) = self
            .schema
            .workflows
            .values()
            .find(|w| w.entity == Symbol::from(entity_name))
            && let Some(obj) = data.as_object_mut()
            && !obj.contains_key(wf.field.as_str())
        {
            obj.insert(wf.field.to_string(), Value::String(wf.initial_state.to_string()));
        }

        // Validation & Transformation (Hashing)
        if let Some(obj) = data.as_object_mut() {
            for field in &entity_schema.fields {
                if field.required && !obj.contains_key(&field.name.to_string()) {
                    return Err(format!("Missing required field: {}", field.name));
                }
            }
            self.process_data_fields(entity_schema, obj)?;
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

        let mut data = data;

        // Workflow Check (Immutability & Transition)
        let workflow = self
            .schema
            .workflows
            .values()
            .find(|w| w.entity == Symbol::from(entity_name));

        if let Some(wf) = workflow {
            let current_record = self.datastore.get(entity_name, id).await?.ok_or("Record not found")?;
            let current_state = current_record
                .get(wf.field.as_str())
                .and_then(|v| v.as_str())
                .unwrap_or("");

            // Check Immutability
            if let Some(state_schema) = wf.states.iter().find(|s| s.name == Symbol::from(current_state)) {
                if state_schema.immutable {
                    if let Some(obj) = data.as_object() {
                        for key in obj.keys() {
                            if key != wf.field.as_str() && key != "id" {
                                return Err(format!(
                                    "Cannot update field '{}' because record is immutable in state '{}'",
                                    key, current_state
                                ));
                            }
                        }
                    }
                }
            }

            if let Some(new_state) = data.get(wf.field.as_str()).and_then(|v| v.as_str()) {
                // Merge data for validation
                let mut merged_record = (*current_record).clone();
                if let Some(target) = merged_record.as_object_mut()
                    && let Some(source) = data.as_object()
                {
                    for (k, v) in source {
                        target.insert(k.clone(), v.clone());
                    }
                }

                // Validate transition logic
                self.workflow
                    .validate_transition(
                        &self.schema,
                        Some(&self.datastore),
                        entity_name,
                        current_state,
                        new_state,
                        &merged_record,
                    )
                    .await
                    .map_err(|e| e.to_string())?;

                // Validate permissions for transition
                if let Some(perm) =
                    self.workflow
                        .get_transition_permission(&self.schema, entity_name, current_state, new_state)
                    && !ctx.has_permission(&perm)
                {
                    return Err(format!("Missing permission '{}' for transition", perm));
                }

                // Apply Side Effects
                let (updates, notifications) =
                    self.workflow
                        .apply_effects(&self.schema, entity_name, current_state, new_state, &merged_record);

                for notification in notifications {
                    println!("NOTIFICATION: {}", notification);
                }

                // Merge updates into data
                if let Some(obj) = data.as_object_mut()
                    && let Value::Object(update_map) = updates
                {
                    for (k, v) in update_map {
                        obj.insert(k, v);
                    }
                }
            }
        }

        // Validation & Transformation (Hashing)
        if let Some(obj) = data.as_object_mut() {
            self.process_data_fields(entity_schema, obj)?;
        }

        self.datastore.update(entity_name, id, data).await
    }

    pub async fn delete(&self, entity_name: &str, id: &str) -> Result<(), String> {
        if !self.schema.entities.contains_key(&Symbol::from(entity_name)) {
            return Err(format!("Entity '{}' not defined", entity_name));
        }
        self.datastore.delete(entity_name, id).await
    }

    fn process_data_fields(
        &self,
        entity_schema: &gurih_ir::EntitySchema,
        obj: &mut serde_json::Map<String, Value>,
    ) -> Result<(), String> {
        for field in &entity_schema.fields {
            if let Some(val) = obj.get_mut(field.name.as_str()) {
                if !crate::validation::validate_type(val, &field.field_type) {
                    return Err(format!("Invalid type for field: {}", field.name));
                }
                // Hash password if applicable
                if field.field_type == FieldType::Password
                    && let Value::String(pass) = val
                {
                    *val = Value::String(hash_password(pass));
                }
            }
        }
        Ok(())
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
