use gurih_ir::{ActionLogic, ActionStep, Symbol};
use std::collections::HashMap;

pub struct ActionEngine {
    actions: HashMap<Symbol, ActionLogic>,
}

impl ActionEngine {
    pub fn new(actions: HashMap<Symbol, ActionLogic>) -> Self {
        Self { actions }
    }

    pub async fn execute(
        &self,
        action_name: &str,
        params: HashMap<String, String>,
        _data_engine: &crate::data::DataEngine, // Will use later
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
            self.execute_step(step, &params, _data_engine).await?;
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

        match step.step_type.as_str() {
            "entity:delete" => {
                // Expects "id" arg
                let id_raw = step.args.get("id").ok_or("Missing 'id' argument for entity:delete")?;
                let id = resolve_arg(id_raw);

                // Call DataEngine delete
                // Assuming data_engine has a delete method
                println!("Executing Delete on {} with ID {}", target_entity, id);
                data_engine
                    .delete(target_entity.as_str(), &id)
                    .await
                    .map_err(|e| e.to_string())?;
            }
            // Add update, create later
            _ => {
                return Err(format!("Unknown step type: {}", step.step_type));
            }
        }

        Ok(())
    }
}

pub struct SimpleResponse {
    pub message: String,
}
