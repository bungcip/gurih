use crate::context::RuntimeContext;
use gurih_ir::{ActionLogic, ActionStep, ActionStepType, Symbol};
use serde_json::json;
use std::collections::HashMap;
use uuid::Uuid;

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
                // Assuming data_engine has a delete method
                println!("Executing Delete on {} with ID {}", target_entity, id);
                data_engine
                    .delete(target_entity.as_str(), &id, ctx)
                    .await
                    .map_err(|e| e.to_string())?;
            }
            ActionStepType::Custom(name) if name == "finance:reverse_journal" => {
                let id_raw = step
                    .args
                    .get("id")
                    .ok_or("Missing 'id' argument for finance:reverse_journal")?;
                let id = resolve_arg(id_raw);

                // 1. Read Original
                let original_arc = data_engine
                    .read("JournalEntry", &id)
                    .await?
                    .ok_or("JournalEntry not found")?;
                let original = original_arc.as_ref();

                // 2. Read Lines
                // Using "journal_entry" as the foreign key field name based on standard snake_case mapping
                let mut filters = HashMap::new();
                filters.insert("journal_entry".to_string(), id.clone());

                let lines = data_engine.datastore().find("JournalLine", filters).await?;

                // 3. Create Reverse Header
                let mut new_entry = original.clone();
                let mut old_entry_number = "?".to_string();

                if let Some(obj) = new_entry.as_object_mut() {
                    // Capture old number for description
                    if let Some(num) = obj.get("entry_number").and_then(|v| v.as_str()) {
                        old_entry_number = num.to_string();
                    }

                    // Clean up system fields
                    obj.remove("id");
                    obj.insert("id".to_string(), json!(Uuid::new_v4().to_string()));
                    obj.remove("entry_number"); // Let serial generator handle it

                    // Update fields
                    obj.insert("status".to_string(), json!("Draft"));
                    obj.insert(
                        "description".to_string(),
                        json!(format!("Reversal of {}", old_entry_number)),
                    );
                    obj.insert("related_journal".to_string(), json!(id));
                }

                let new_id = data_engine.create("JournalEntry", new_entry, ctx).await?;

                // 4. Create Reverse Lines
                for line_arc in lines {
                    let mut line = line_arc.as_ref().clone();
                    if let Some(obj) = line.as_object_mut() {
                        obj.remove("id");
                        obj.insert("id".to_string(), json!(Uuid::new_v4().to_string()));
                        obj.insert("journal_entry".to_string(), json!(new_id));

                        // Swap debit/credit
                        let get_val = |v: &serde_json::Value| -> f64 {
                            if let Some(f) = v.as_f64() {
                                f
                            } else if let Some(s) = v.as_str() {
                                s.parse().unwrap_or(0.0)
                            } else {
                                0.0
                            }
                        };

                        let debit = obj.get("debit").map(get_val).unwrap_or(0.0);
                        let credit = obj.get("credit").map(get_val).unwrap_or(0.0);

                        // Store as string for Money type compatibility
                        obj.insert("debit".to_string(), json!(credit.to_string()));
                        obj.insert("credit".to_string(), json!(debit.to_string()));
                    }
                    data_engine.create("JournalLine", line, ctx).await?;
                }
            }
            // Add update, create later if needed by IR
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
