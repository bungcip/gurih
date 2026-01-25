use crate::constants::{
    DEFAULT_BIRTH_DATE_FIELD, DEFAULT_JOIN_DATE_FIELD, FIELD_IS_PAYROLL_ACTIVE, FIELD_RANK_ELIGIBLE,
};
use crate::errors::RuntimeError;
use chrono::NaiveDate;
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
                    self.check_precondition(pre, entity_data)?;
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

    fn check_precondition(&self, pre: &TransitionPrecondition, entity_data: &Value) -> Result<(), RuntimeError> {
        match pre {
            TransitionPrecondition::Document(doc_name) => {
                let has_doc = entity_data
                    .get(doc_name.as_str())
                    .map(|v| !v.is_null() && !v.as_str().unwrap_or("").is_empty())
                    .unwrap_or(false);
                if !has_doc {
                    return Err(RuntimeError::ValidationError(format!(
                        "Missing required document: {}",
                        doc_name
                    )));
                }
            }
            TransitionPrecondition::MinYearsOfService { years, from_field } => {
                let field_name = from_field
                    .as_ref()
                    .map(|s| s.as_str())
                    .unwrap_or(DEFAULT_JOIN_DATE_FIELD);

                let join_date_str = entity_data.get(field_name).and_then(|v| v.as_str());

                if let Some(date_str) = join_date_str {
                    if !check_min_years(date_str, *years) {
                        return Err(RuntimeError::ValidationError(format!(
                            "Minimum {} years of service required",
                            years
                        )));
                    }
                } else {
                    return Err(RuntimeError::ValidationError(format!(
                        "Cannot determine years of service (missing '{}')",
                        field_name
                    )));
                }
            }
            TransitionPrecondition::MinAge { age, birth_date_field } => {
                let field_name = birth_date_field
                    .as_ref()
                    .map(|s| s.as_str())
                    .unwrap_or(DEFAULT_BIRTH_DATE_FIELD);

                let birth_date_str = entity_data.get(field_name).and_then(|v| v.as_str());

                if let Some(date_str) = birth_date_str {
                    if !check_min_years(date_str, *age) {
                        return Err(RuntimeError::ValidationError(format!(
                            "Minimum age of {} required",
                            age
                        )));
                    }
                } else {
                    return Err(RuntimeError::ValidationError(format!(
                        "Cannot determine age (missing '{}')",
                        field_name
                    )));
                }
            }
            TransitionPrecondition::ValidEffectiveDate(field) => {
                let date_val = entity_data.get(field.as_str());
                match date_val {
                    Some(Value::String(s)) => {
                        if NaiveDate::parse_from_str(s, "%Y-%m-%d").is_err() {
                            return Err(RuntimeError::ValidationError(format!(
                                "Field '{}' must be a valid date (YYYY-MM-DD)",
                                field
                            )));
                        }
                    }
                    _ => {
                        return Err(RuntimeError::ValidationError(format!(
                            "Missing or invalid effective date in field '{}'",
                            field
                        )));
                    }
                }
            }
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

        if let Some(wf) = workflow
            && let Some(t) = wf
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
                    TransitionEffect::UpdateRankEligibility(active) => {
                        updates.insert(FIELD_RANK_ELIGIBLE.to_string(), Value::Bool(*active));
                    }
                    TransitionEffect::UpdateField { field, value } => {
                        updates.insert(field.to_string(), Value::String(value.clone()));
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
