use crate::context::RuntimeContext;
use crate::plugins::Plugin;
use gurih_ir::{ActionLogic, ActionStep, ActionStepType, Symbol};
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

        // Helper to resolve args from params
        let resolve_arg = |val: &str| -> String {
            if val.starts_with("param(") && val.ends_with(")") {
                let key = &val[6..val.len() - 1];
                let cleaned_key = key.trim_matches('"');
                params.get(cleaned_key).cloned().unwrap_or(val.to_string())
            } else {
                val.to_string()
            }
        };

        match &step.step_type {
            ActionStepType::EntityDelete => {
                // Expects "id" arg
                let id_raw = step.args.get("id").ok_or("Missing 'id' argument for entity:delete")?;
                let id = resolve_arg(id_raw);

                // Call DataEngine delete
                println!("Executing Delete on {} with ID {}", target_entity, id);
                data_engine
                    .delete(target_entity.as_str(), &id, ctx)
                    .await
                    .map_err(|e| e.to_string())?;
            }
            ActionStepType::Custom(name) => {
                let mut handled = false;
                for plugin in &self.plugins {
                    if plugin
                        .execute_action_step(name, step, params, data_engine, ctx)
                        .await
                        .map_err(|e| e.to_string())?
                    {
                        handled = true;
                        break;
                    }
                }
                if !handled {
                    println!("Action step custom '{}' not handled by any plugin", name);
                    // Optionally return error if strict
                    // return Err(format!("Action step custom '{}' not handled", name));
                }
            }
            _ => {
                println!("Step type {:?} not yet implemented in ActionEngine", step.step_type);
            }
        }

        Ok(())
    }
}

pub struct SimpleResponse {
    pub message: String,
}
