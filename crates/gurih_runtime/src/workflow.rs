use crate::datastore::DataStore;
use crate::errors::RuntimeError;
use chrono::{Datelike, NaiveDate, Utc};
use gurih_ir::{FieldType, Schema, Symbol, TransitionEffect, TransitionPrecondition};
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
            TransitionPrecondition::Assertion(expr) => {
                let result = crate::evaluator::evaluate(expr, entity_data)?;
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
                                if let Some(line_obj) = line.as_object()
                                    && (line_obj.contains_key("debit") || line_obj.contains_key("credit"))
                                {
                                    is_journal_line = true;
                                    let debit = line_obj
                                        .get("debit")
                                        .and_then(|v| v.as_str())
                                        .unwrap_or("0")
                                        .parse::<f64>()
                                        .unwrap_or(0.0)
                                        + line_obj.get("debit").and_then(|v| v.as_f64()).unwrap_or(0.0);
                                    let credit = line_obj
                                        .get("credit")
                                        .and_then(|v| v.as_str())
                                        .unwrap_or("0")
                                        .parse::<f64>()
                                        .unwrap_or(0.0)
                                        + line_obj.get("credit").and_then(|v| v.as_f64()).unwrap_or(0.0);

                                    total_debit += debit;
                                    total_credit += credit;
                                }
                            }

                            if is_journal_line {
                                found_lines = true;
                                let diff = (total_debit - total_credit).abs();
                                if diff > 0.01 {
                                    // epsilon
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
                    let date_str = entity_data
                        .get("date")
                        .or_else(|| entity_data.get("transaction_date"))
                        .and_then(|v| v.as_str());

                    if let Some(date_s) = date_str {
                        // Strict validation
                        if NaiveDate::parse_from_str(date_s, "%Y-%m-%d").is_err() {
                            return Err(RuntimeError::ValidationError(format!(
                                "Invalid date format: {}",
                                date_s
                            )));
                        }

                        let target_entity = entity.as_ref().map(|s| s.as_str()).unwrap_or("AccountingPeriod");

                        // Validate identifier
                        if !target_entity.chars().all(|c| c.is_alphanumeric() || c == '_') {
                            return Err(RuntimeError::WorkflowError(format!(
                                "Invalid entity name for period check: {}",
                                target_entity
                            )));
                        }

                        let table_name = target_entity.to_lowercase(); // Simple heuristic for now, safe with alphanumeric check

                        // Construct SQL
                        let sql = format!(
                            "SELECT id FROM {} WHERE status = 'Open' AND start_date <= '{}' AND end_date >= '{}'",
                            table_name, date_s, date_s
                        );

                        let periods = ds.query(&sql).await.map_err(RuntimeError::WorkflowError)?;
                        if periods.is_empty() {
                            return Err(RuntimeError::ValidationError(format!(
                                "No open {} found for date {}",
                                target_entity, date_s
                            )));
                        }
                    } else {
                        return Err(RuntimeError::ValidationError(
                            "Missing date field for period check".to_string(),
                        ));
                    }
                } else {
                    return Err(RuntimeError::WorkflowError(
                        "Cannot check PeriodOpen: Datastore not available".to_string(),
                    ));
                }
            }
            TransitionPrecondition::Document(doc_name) => {
                let field_name = doc_name.as_str();
                match entity_data.get(field_name) {
                    Some(Value::String(s)) if !s.is_empty() => {} // OK
                    Some(Value::Null) | None => {
                        return Err(RuntimeError::ValidationError(format!(
                            "Missing required document: '{}'",
                            doc_name
                        )));
                    }
                    _ => {
                        return Err(RuntimeError::ValidationError(format!(
                            "Invalid format for document field '{}'",
                            doc_name
                        )));
                    }
                }
            }
            TransitionPrecondition::MinYearsOfService { years, from_field } => {
                let field_name = from_field.as_ref().map(|s| s.as_str()).unwrap_or("join_date");
                let date_str = entity_data
                    .get(field_name)
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| {
                        RuntimeError::ValidationError(format!("Missing date field '{}' for service check", field_name))
                    })?;

                let join_date = NaiveDate::parse_from_str(date_str, "%Y-%m-%d")
                    .map_err(|_| RuntimeError::ValidationError(format!("Invalid date format in '{}'", field_name)))?;
                let now = Utc::now().date_naive();

                let mut service_years = now.year() - join_date.year();
                if (now.month(), now.day()) < (join_date.month(), join_date.day()) {
                    service_years -= 1;
                }

                if service_years < *years as i32 {
                    return Err(RuntimeError::ValidationError(format!(
                        "Insufficient years of service: Has {}, requires {}",
                        service_years, years
                    )));
                }
            }
            TransitionPrecondition::MinAge {
                age,
                birth_date_field,
            } => {
                let field_name = birth_date_field
                    .as_ref()
                    .map(|s| s.as_str())
                    .unwrap_or("birth_date");
                let date_str = entity_data
                    .get(field_name)
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| {
                        RuntimeError::ValidationError(format!("Missing date field '{}' for age check", field_name))
                    })?;

                let birth_date = NaiveDate::parse_from_str(date_str, "%Y-%m-%d")
                    .map_err(|_| RuntimeError::ValidationError(format!("Invalid date format in '{}'", field_name)))?;
                let now = Utc::now().date_naive();

                let mut current_age = now.year() - birth_date.year();
                if (now.month(), now.day()) < (birth_date.month(), birth_date.day()) {
                    current_age -= 1;
                }

                if current_age < *age as i32 {
                    return Err(RuntimeError::ValidationError(format!(
                        "Minimum age not met: Is {}, requires {}",
                        current_age, age
                    )));
                }
            }
            TransitionPrecondition::ValidEffectiveDate(field_name) => {
                let date_str = entity_data
                    .get(field_name.as_str())
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| {
                        RuntimeError::ValidationError(format!("Missing effective date field '{}'", field_name))
                    })?;

                if NaiveDate::parse_from_str(date_str, "%Y-%m-%d").is_err() {
                    return Err(RuntimeError::ValidationError(format!(
                        "Invalid effective date format in '{}'",
                        field_name
                    )));
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
                    TransitionEffect::SuspendPayroll(active) => {
                        updates.insert("is_payroll_active".to_string(), Value::Bool(*active));
                    }
                    TransitionEffect::UpdateRankEligibility(active) => {
                        updates.insert("rank_eligible".to_string(), Value::Bool(*active));
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

#[cfg(test)]
mod tests {
    use super::*;
    use gurih_ir::{StateSchema, Transition, TransitionPrecondition, TransitionEffect, WorkflowSchema};
    use serde_json::json;
    use std::collections::HashMap;
    use chrono::Utc;

    #[tokio::test]
    async fn test_workflow_diagnostics() {
        let engine = WorkflowEngine::new();

        // Define a workflow manually
        let workflow = WorkflowSchema {
            name: Symbol::from("EmployeeStatusWorkflow"),
            entity: Symbol::from("Employee"),
            field: Symbol::from("status"),
            initial_state: Symbol::from("CPNS"),
            states: vec![
                StateSchema { name: Symbol::from("CPNS"), immutable: false },
                StateSchema { name: Symbol::from("PNS"), immutable: false },
            ],
            transitions: vec![
                Transition {
                    name: Symbol::from("promote"),
                    from: Symbol::from("CPNS"),
                    to: Symbol::from("PNS"),
                    required_permission: None,
                    preconditions: vec![
                        TransitionPrecondition::Document(Symbol::from("sk_pns")),
                        TransitionPrecondition::MinYearsOfService {
                            years: 1,
                            from_field: Some(Symbol::from("join_date")),
                        }
                    ],
                    effects: vec![
                        TransitionEffect::UpdateRankEligibility(true)
                    ],
                }
            ]
        };

        let mut workflows = HashMap::new();
        workflows.insert(Symbol::from("EmployeeStatusWorkflow"), workflow);

        // Mock Schema
        let mut schema = Schema::default();
        schema.workflows = workflows;

        // Test 1: Missing Document
        let data_missing_doc = json!({
            "status": "CPNS",
            "join_date": "2020-01-01"
        });

        let res = engine.validate_transition(&schema, None, "Employee", "CPNS", "PNS", &data_missing_doc).await;
        assert!(res.is_err());
        let err_msg = res.unwrap_err().to_string();
        assert!(err_msg.contains("Missing required document: 'sk_pns'"), "Got: {}", err_msg);

        // Test 2: Document Present, Service Insufficient (Assume current year is e.g. 2024, join date 2024 -> 0 years)
        // To make this test deterministic without mocking Utc::now(), we should set join_date to Today
        let today = Utc::now().date_naive().format("%Y-%m-%d").to_string();
        let data_insufficient_service = json!({
            "status": "CPNS",
            "sk_pns": "path/to/file.pdf",
            "join_date": today
        });

        let res = engine.validate_transition(&schema, None, "Employee", "CPNS", "PNS", &data_insufficient_service).await;
        assert!(res.is_err());
        let err_msg = res.unwrap_err().to_string();
        assert!(err_msg.contains("Insufficient years of service"), "Got: {}", err_msg);

        // Test 3: Success (Join date 10 years ago)
        let data_success = json!({
            "status": "CPNS",
            "sk_pns": "path/to/file.pdf",
            "join_date": "2010-01-01"
        });

        let res = engine.validate_transition(&schema, None, "Employee", "CPNS", "PNS", &data_success).await;
        assert!(res.is_ok());

        // Test Effects
        let (updates, _) = engine.apply_effects(&schema, "Employee", "CPNS", "PNS", &data_success);
        assert_eq!(updates.get("rank_eligible"), Some(&Value::Bool(true)));
    }
}
