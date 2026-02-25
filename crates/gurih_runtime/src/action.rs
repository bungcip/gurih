use crate::context::RuntimeContext;
use crate::plugins::Plugin;
use gurih_ir::utils::resolve_param;
use gurih_ir::{ActionLogic, ActionStep, ActionStepType, FieldType, Symbol};
use serde_json::Value;
use std::collections::HashMap;

pub struct ActionEngine {
    actions: HashMap<Symbol, ActionLogic>,
    plugins: Vec<Box<dyn Plugin>>,
}

impl ActionEngine {
    pub fn new(actions: HashMap<Symbol, ActionLogic>) -> Self {
        Self {
            actions,
            plugins: vec![],
        }
    }

    pub fn with_plugins(mut self, plugins: Vec<Box<dyn Plugin>>) -> Self {
        self.plugins = plugins;
        self
    }

    pub async fn execute(
        &self,
        action_name: &str,
        params: HashMap<String, String>,
        data_engine: &crate::data::DataEngine,
        ctx: &RuntimeContext,
    ) -> Result<SimpleResponse, String> {
        // Use simple result for now
        let action = self
            .actions
            .get(&Symbol::from(action_name))
            .ok_or_else(|| format!("Action not found: {}", action_name))?;

        // 1. Validate params
        // For now, assume all provided.

        // 2. Execute steps
        for step in &action.steps {
            self.execute_step(step, &params, data_engine, ctx).await?;
        }

        Ok(SimpleResponse {
            message: format!("Action {} executed successfully", action_name),
        })
    }

    async fn execute_step(
        &self,
        step: &ActionStep,
        params: &HashMap<String, String>,
        data_engine: &crate::data::DataEngine,
        ctx: &RuntimeContext,
    ) -> Result<(), String> {
        let target_entity = &step.target;

        match &step.step_type {
            ActionStepType::EntityDelete => {
                // Expects "id" arg
                let id_raw = step.args.get("id").ok_or("Missing 'id' argument for entity:delete")?;
                let id = resolve_param(id_raw, params);

                // Call DataEngine delete
                data_engine
                    .delete(target_entity.as_str(), &id, ctx)
                    .await
                    .map_err(|e| e.to_string())?;
            }
            ActionStepType::EntityUpdate => {
                let id_raw = step.args.get("id").ok_or("Missing 'id' argument for entity:update")?;
                let id = resolve_param(id_raw, params);

                let schema = data_engine.get_schema();
                let entity_schema = schema
                    .entities
                    .get(target_entity)
                    .ok_or_else(|| format!("Entity not found: {}", target_entity))?;

                let mut update_data = serde_json::Map::new();

                for (key, val_raw) in &step.args {
                    if key == "id" {
                        continue;
                    }
                    let val_str = resolve_param(val_raw, params);

                    // Find field type to convert
                    if let Some(field) = entity_schema.fields.iter().find(|f| f.name == Symbol::from(key)) {
                        let value = match field.field_type {
                            FieldType::Integer => val_str
                                .parse::<i64>()
                                .map(Value::from)
                                .map_err(|_| format!("Invalid integer for field {}: {}", key, val_str))?,
                            FieldType::Float => val_str
                                .parse::<f64>()
                                .map(Value::from)
                                .map_err(|_| format!("Invalid float for field {}: {}", key, val_str))?,
                            FieldType::Boolean => val_str
                                .parse::<bool>()
                                .map(Value::Bool)
                                .map_err(|_| format!("Invalid boolean for field {}: {}", key, val_str))?,
                            _ => Value::String(val_str),
                        };
                        update_data.insert(key.clone(), value);
                    } else {
                        // If not in schema, just pass as string
                        update_data.insert(key.clone(), Value::String(val_str));
                    }
                }

                data_engine
                    .update(target_entity.as_str(), &id, Value::Object(update_data), ctx)
                    .await
                    .map_err(|e| e.to_string())?;
            }
            ActionStepType::Custom(name) => {
                for plugin in &self.plugins {
                    if plugin
                        .execute_action_step(name, step, params, data_engine, ctx)
                        .await
                        .map_err(|e| e.to_string())?
                    {
                        break;
                    }
                }
            }
        }

        Ok(())
    }
}

pub struct SimpleResponse {
    pub message: String,
}
