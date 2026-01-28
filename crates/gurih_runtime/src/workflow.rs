use crate::datastore::DataStore;
use crate::errors::RuntimeError;
use crate::plugins::WorkflowPlugin;
use gurih_ir::{FieldType, Schema, Symbol, TransitionEffect, TransitionPrecondition};
use serde_json::Value;
use std::sync::Arc;

pub struct WorkflowEngine {
    plugins: Vec<Box<dyn WorkflowPlugin>>,
}

impl Default for WorkflowEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl WorkflowEngine {
    pub fn new() -> Self {
        Self { plugins: vec![] }
    }

    pub fn with_plugins(mut self, plugins: Vec<Box<dyn WorkflowPlugin>>) -> Self {
        self.plugins = plugins;
        self
    }

    pub async fn validate_transition(
        &self,
        schema: &Schema,
        datastore: Option<&Arc<dyn DataStore>>,
        entity_name: &str,
        current_state: &str,
        new_state: &str,
        entity_data: &Value,
    ) -> Result<(), RuntimeError> {
        // Find workflow for entity
        let workflow = schema
            .workflows
            .values()
            .find(|w| w.entity == Symbol::from(entity_name));

        if let Some(wf) = workflow {
            // If staying in same state, it's usually allowed (update other fields)
            if current_state == new_state {
                return Ok(());
            }

            // Check if transition exists from current to new
            let transition = wf
                .transitions
                .iter()
                .find(|t| t.from == Symbol::from(current_state) && t.to == Symbol::from(new_state));

            if let Some(t) = transition {
                // Check Preconditions
                for pre in &t.preconditions {
                    self.check_precondition(pre, entity_data, schema, datastore).await?;
                }
                return Ok(());
            }

            return Err(RuntimeError::WorkflowError(format!(
                "Invalid transition from '{}' to '{}' for entity '{}'",
                current_state, new_state, entity_name
            )));
        }

        Ok(())
    }

    async fn check_precondition(
        &self,
        pre: &TransitionPrecondition,
        entity_data: &Value,
        schema: &Schema,
        datastore: Option<&Arc<dyn DataStore>>,
    ) -> Result<(), RuntimeError> {
        match pre {
            TransitionPrecondition::Assertion(expr) => {
                let result = crate::evaluator::evaluate(expr, entity_data, Some(schema), datastore).await?;
                match result {
                    Value::Bool(true) => {}
                    Value::Bool(false) => {
                        return Err(RuntimeError::ValidationError(
                            "Transition condition not met".to_string(),
                        ));
                    }
                    _ => {
                        return Err(RuntimeError::WorkflowError(
                            "Assertion expression must evaluate to boolean".to_string(),
                        ));
                    }
                }
            }
            TransitionPrecondition::Custom { name, args } => {
                for plugin in &self.plugins {
                    // We call check_precondition on all plugins.
                    // The contract is: if plugin recognizes it, checks it.
                    // If it passes or unknown, returns Ok. If fails, returns Err.
                    // We assume that if ANY plugin fails, the whole thing fails.
                    // But we also need to know if at least one plugin recognized it?
                    // The current trait returns Result<(), Error>. This implies "Pass or Ignore".
                    // If a plugin ignores it (Ok), and no plugin handles it, should we fail?
                    // For now, let's assume plugins handle what they know. If unknown, we ignore (maybe it's a client-side rule?)
                    // Or we should warn "Unknown rule".
                    // Let's stick to simple "Try all plugins, fail if any errors".
                    plugin.check_precondition(name.as_str(), args, entity_data, schema, datastore).await?;
                }
                // If not handled by any plugin, strictly speaking we should probably allow it (maybe implemented elsewhere or future)
                // or fail.
            }
        }
        Ok(())
    }

    pub async fn apply_effects(
        &self,
        schema: &Schema,
        entity_name: &str,
        current_state: &str,
        new_state: &str,
        entity_data: &Value,
    ) -> (Value, Vec<String>, Vec<Symbol>) {
        let mut updates = serde_json::Map::new();
        let mut notifications = vec![];
        let mut postings = vec![];

        let workflow = schema
            .workflows
            .values()
            .find(|w| w.entity == Symbol::from(entity_name));

        if let Some(wf) = workflow
            && let Some(t) = wf
                .transitions
                .iter()
                .find(|t| t.from == Symbol::from(current_state) && t.to == Symbol::from(new_state))
        {
            for effect in &t.effects {
                match effect {
                    TransitionEffect::Notify(target) => {
                        notifications.push(target.to_string());
                    }
                    TransitionEffect::UpdateField { field, value } => {
                        let mut json_val = Value::String(value.clone());

                        // Attempt to cast to correct type if entity definition is available
                        if let Some(ent) = schema.entities.get(&Symbol::from(entity_name))
                            && let Some(f_def) = ent.fields.iter().find(|f| f.name == *field)
                            && f_def.field_type == FieldType::Boolean
                            && let Ok(b) = value.parse::<bool>()
                        {
                            json_val = Value::Bool(b);
                        }

                        updates.insert(field.to_string(), json_val);
                    }
                    TransitionEffect::Custom { name, args } => {
                        for plugin in &self.plugins {
                            if let Ok((p_updates, p_notifications, p_postings)) = plugin.apply_effect(name.as_str(), args, schema, entity_name, entity_data).await {
                                // Merge results
                                if let Value::Object(obj) = p_updates {
                                    for (k, v) in obj {
                                        updates.insert(k, v);
                                    }
                                }
                                notifications.extend(p_notifications);
                                postings.extend(p_postings);
                            }
                        }
                    }
                }
            }
        }

        (Value::Object(updates), notifications, postings)
    }

    pub fn get_initial_state(&self, schema: &Schema, entity_name: &str) -> Option<String> {
        schema
            .workflows
            .values()
            .find(|w| w.entity == Symbol::from(entity_name))
            .map(|w| w.initial_state.to_string())
    }

    pub fn get_transition_permission(
        &self,
        schema: &Schema,
        entity_name: &str,
        current_state: &str,
        new_state: &str,
    ) -> Option<String> {
        if current_state == new_state {
            return None;
        }

        let workflow = schema
            .workflows
            .values()
            .find(|w| w.entity == Symbol::from(entity_name))?;
        workflow
            .transitions
            .iter()
            .find(|t| t.from == Symbol::from(current_state) && t.to == Symbol::from(new_state))
            .and_then(|t| t.required_permission.as_ref().map(|s: &Symbol| s.to_string()))
    }
}
