use crate::constants::{
    DEFAULT_BIRTH_DATE_FIELD, DEFAULT_JOIN_DATE_FIELD, FIELD_IS_PAYROLL_ACTIVE, FIELD_RANK_ELIGIBLE,
};
use crate::datastore::DataStore;
use crate::errors::RuntimeError;
use chrono::NaiveDate;
use gurih_common::time::check_min_years;
use gurih_ir::{Schema, Symbol, TransitionEffect, TransitionPrecondition};
use serde_json::Value;
use std::sync::Arc;

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
                    self.check_precondition(pre, entity_data, datastore).await?;
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
        datastore: Option<&Arc<dyn DataStore>>,
    ) -> Result<(), RuntimeError> {
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
            TransitionPrecondition::BalancedTransaction => {
                // Find fields that are arrays (composition/child tables)
                if let Some(obj) = entity_data.as_object() {
                    let mut found_lines = false;
                    for (_key, val) in obj {
                        if let Some(lines) = val.as_array() {
                            // Heuristic: check if items have debit/credit
                            // Or assuming user names it something like "lines" or "items"?
                            // Let's assume ANY array with debit/credit is a journal line list.
                            let mut total_debit = 0.0;
                            let mut total_credit = 0.0;
                            let mut is_journal_line = false;

                            for line in lines {
                                if let Some(line_obj) = line.as_object() {
                                    if line_obj.contains_key("debit") || line_obj.contains_key("credit") {
                                        is_journal_line = true;
                                        let debit = line_obj.get("debit").and_then(|v| v.as_str()).unwrap_or("0").parse::<f64>().unwrap_or(0.0)
                                            + line_obj.get("debit").and_then(|v| v.as_f64()).unwrap_or(0.0);
                                        let credit = line_obj.get("credit").and_then(|v| v.as_str()).unwrap_or("0").parse::<f64>().unwrap_or(0.0)
                                            + line_obj.get("credit").and_then(|v| v.as_f64()).unwrap_or(0.0);

                                        total_debit += debit;
                                        total_credit += credit;
                                    }
                                }
                            }

                            if is_journal_line {
                                found_lines = true;
                                let diff = (total_debit - total_credit).abs();
                                if diff > 0.01 { // epsilon
                                    return Err(RuntimeError::ValidationError(format!(
                                        "Transaction not balanced: Debit {}, Credit {} (Diff {})",
                                        total_debit, total_credit, diff
                                    )));
                                }
                            }
                        }
                    }
                    if !found_lines {
                         // Should we fail if no lines? A journal without lines is technically balanced (0=0), but maybe useless.
                         // Let's allow it for now, or fail?
                         // "Balanced Transaction" implies there is a transaction.
                         // But if 0=0, it's balanced.
                    }
                }
            }
            TransitionPrecondition::PeriodOpen { entity } => {
                // Needs datastore access
                if let Some(ds) = datastore {
                    // 1. Get transaction date
                    let date_str = entity_data.get("date").or_else(|| entity_data.get("transaction_date"))
                        .and_then(|v| v.as_str());

                    if let Some(date_s) = date_str {
                        // Strict validation
                        if NaiveDate::parse_from_str(date_s, "%Y-%m-%d").is_err() {
                             return Err(RuntimeError::ValidationError(format!("Invalid date format: {}", date_s)));
                        }

                        let target_entity = entity.as_ref().map(|s| s.as_str()).unwrap_or("AccountingPeriod");

                        // Validate identifier
                        if !target_entity.chars().all(|c| c.is_alphanumeric() || c == '_') {
                             return Err(RuntimeError::WorkflowError(format!("Invalid entity name for period check: {}", target_entity)));
                        }

                        // Determine table name
                        // Note: We don't have access to full schema.tables easily unless passed, but we have `schema`.
                        // self.validate_transition receives &Schema.
                        // But check_precondition doesn't receive &Schema in my previous edit?
                        // Let's check check_precondition signature.

                        let table_name = target_entity.to_lowercase(); // Simple heuristic for now, safe with alphanumeric check

                        // Construct SQL
                        let sql = format!(
                            "SELECT id FROM {} WHERE status = 'Open' AND start_date <= '{}' AND end_date >= '{}'",
                            table_name, date_s, date_s
                        );

                        let periods = ds.query(&sql).await.map_err(|e| RuntimeError::WorkflowError(e))?;
                        if periods.is_empty() {
                            return Err(RuntimeError::ValidationError(format!(
                                "No open {} found for date {}", target_entity, date_s
                            )));
                        }
                    } else {
                         return Err(RuntimeError::ValidationError("Missing date field for period check".to_string()));
                    }
                } else {
                    return Err(RuntimeError::WorkflowError("Cannot check PeriodOpen: Datastore not available".to_string()));
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
