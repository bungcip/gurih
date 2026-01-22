use crate::constants::{FIELD_IS_PAYROLL_ACTIVE, FIELD_JOIN_DATE, FIELD_TMT_CPNS};
use gurih_common::time::check_min_years;
use gurih_ir::{Schema, Symbol, TransitionEffect, TransitionPrecondition};
use serde_json::Value;

pub struct WorkflowEngine;

impl Default for WorkflowEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl WorkflowEngine {
    pub fn new() -> Self {
        Self
    }

    pub fn validate_transition(
        &self,
        schema: &Schema,
        entity_name: &str,
        current_state: &str,
        new_state: &str,
        entity_data: &Value,
    ) -> Result<(), String> {
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
                    match pre {
                        TransitionPrecondition::Document(doc_name) => {
                            let has_doc = entity_data
                                .get(doc_name.as_str())
                                .map(|v| !v.is_null() && v.as_str().unwrap_or("") != "")
                                .unwrap_or(false);
                            if !has_doc {
                                return Err(format!("Missing required document: {}", doc_name));
                            }
                        }
                        TransitionPrecondition::MinYearsOfService(min_years) => {
                            let join_date_str = entity_data
                                .get(FIELD_TMT_CPNS)
                                .or_else(|| entity_data.get(FIELD_JOIN_DATE))
                                .and_then(|v| v.as_str());

                            if let Some(date_str) = join_date_str {
                                if !check_min_years(date_str, *min_years) {
                                    return Err(format!("Minimum {} years of service required", min_years));
                                }
                            } else {
                                return Err(format!(
                                    "Cannot determine years of service (missing '{}' or '{}')",
                                    FIELD_TMT_CPNS, FIELD_JOIN_DATE
                                ));
                            }
                        }
                    }
                }
                return Ok(());
            }

            return Err(format!(
                "Invalid transition from '{}' to '{}' for entity '{}'",
                current_state, new_state, entity_name
            ));
        }

        Ok(())
    }

    pub fn apply_effects(
        &self,
        schema: &Schema,
        entity_name: &str,
        current_state: &str,
        new_state: &str,
        _entity_data: &Value,
    ) -> (Value, Vec<String>) {
        let mut updates = serde_json::Map::new();
        let mut notifications = vec![];

        let workflow = schema
            .workflows
            .values()
            .find(|w| w.entity == Symbol::from(entity_name));

        if let Some(wf) = workflow {
            if let Some(t) = wf
                .transitions
                .iter()
                .find(|t| t.from == Symbol::from(current_state) && t.to == Symbol::from(new_state))
            {
                for effect in &t.effects {
                    match effect {
                        TransitionEffect::SuspendPayroll(active) => {
                            updates.insert(FIELD_IS_PAYROLL_ACTIVE.to_string(), Value::Bool(*active));
                        }
                        TransitionEffect::Notify(target) => {
                            notifications.push(target.to_string());
                        }
                    }
                }
            }
        }

        (Value::Object(updates), notifications)
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

