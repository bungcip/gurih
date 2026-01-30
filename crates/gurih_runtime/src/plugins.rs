use crate::context::RuntimeContext;
use crate::datastore::DataStore;
use crate::errors::RuntimeError;
use crate::traits::DataAccess;
use async_trait::async_trait;
use chrono::NaiveDate;
use gurih_ir::{ActionStep, Expression, Schema, Symbol};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;

#[async_trait]
pub trait Plugin: Send + Sync {
    fn name(&self) -> &str;

    /// Checks a custom precondition.
    async fn check_precondition(
        &self,
        name: &str,
        args: &[Expression],
        entity_data: &Value,
        schema: &Schema,
        datastore: Option<&Arc<dyn DataStore>>,
    ) -> Result<(), RuntimeError>;

    /// Applies a custom effect.
    async fn apply_effect(
        &self,
        name: &str,
        args: &[Expression],
        schema: &Schema,
        entity_name: &str,
        entity_data: &Value,
    ) -> Result<(Value, Vec<String>, Vec<Symbol>), RuntimeError>;

    /// Executes a custom action step.
    /// Returns Ok(true) if handled, Ok(false) if not recognized.
    async fn execute_action_step(
        &self,
        step_name: &str,
        step: &ActionStep,
        params: &HashMap<String, String>,
        data_access: &dyn DataAccess,
        ctx: &RuntimeContext,
    ) -> Result<bool, RuntimeError>;
}

pub struct FinancePlugin;

#[async_trait]
impl Plugin for FinancePlugin {
    fn name(&self) -> &str {
        "FinancePlugin"
    }

    async fn check_precondition(
        &self,
        name: &str,
        args: &[Expression],
        entity_data: &Value,
        _schema: &Schema,
        datastore: Option<&Arc<dyn DataStore>>,
    ) -> Result<(), RuntimeError> {
        match name {
            "balanced_transaction" => {
                if let Some(obj) = entity_data.as_object() {
                    for (_key, val) in obj {
                        if let Some(lines) = val.as_array() {
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
                                let diff = (total_debit - total_credit).abs();
                                if diff > 0.01 {
                                    return Err(RuntimeError::ValidationError(format!(
                                        "Transaction not balanced: Debit {}, Credit {} (Diff {})",
                                        total_debit, total_credit, diff
                                    )));
                                }
                            }
                        }
                    }
                }
                Ok(())
            }
            "period_open" => {
                if let Some(ds) = datastore {
                    let date_str = entity_data
                        .get("date")
                        .or_else(|| entity_data.get("transaction_date"))
                        .and_then(|v| v.as_str());

                    if let Some(date_s) = date_str {
                        if NaiveDate::parse_from_str(date_s, "%Y-%m-%d").is_err() {
                            return Err(RuntimeError::ValidationError(format!(
                                "Invalid date format: {}",
                                date_s
                            )));
                        }

                        let target_entity = if let Some(Expression::StringLiteral(s)) = args.first() {
                            s.as_str()
                        } else {
                            "AccountingPeriod"
                        };

                        if !target_entity.chars().all(|c| c.is_alphanumeric() || c == '_') {
                            return Err(RuntimeError::WorkflowError(format!(
                                "Invalid entity name for period check: {}",
                                target_entity
                            )));
                        }

                        let table_name = target_entity.to_lowercase();
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
                Ok(())
            }
            _ => Ok(()),
        }
    }

    async fn apply_effect(
        &self,
        name: &str,
        args: &[Expression],
        _schema: &Schema,
        _entity_name: &str,
        _entity_data: &Value,
    ) -> Result<(Value, Vec<String>, Vec<Symbol>), RuntimeError> {
        if name == "post_journal" {
            if let Some(Expression::StringLiteral(rule)) = args.first() {
                return Ok((Value::Null, vec![], vec![Symbol::from(rule.as_str())]));
            }
        }
        Ok((Value::Null, vec![], vec![]))
    }

    async fn execute_action_step(
        &self,
        step_name: &str,
        step: &ActionStep,
        params: &HashMap<String, String>,
        data_access: &dyn DataAccess,
        ctx: &RuntimeContext,
    ) -> Result<bool, RuntimeError> {
        if step_name == "finance:reverse_journal" {
            let resolve_arg = |val: &str| -> String {
                if val.starts_with("param(") && val.ends_with(")") {
                    let key = &val[6..val.len() - 1];
                    let cleaned_key = key.trim_matches('"');
                    params.get(cleaned_key).cloned().unwrap_or(val.to_string())
                } else {
                    val.to_string()
                }
            };

            let id_raw = step
                .args
                .get("id")
                .ok_or(RuntimeError::WorkflowError(
                    "Missing 'id' argument for finance:reverse_journal".to_string(),
                ))?;
            let id = resolve_arg(id_raw);

            // 1. Read Original
            let original_arc = data_access
                .read("JournalEntry", &id)
                .await
                .map_err(RuntimeError::WorkflowError)?
                .ok_or(RuntimeError::WorkflowError("JournalEntry not found".to_string()))?;
            let original = original_arc.as_ref();

            // 2. Read Lines
            let mut filters = HashMap::new();
            filters.insert("journal_entry".to_string(), id.clone());

            let schema = data_access.get_schema();
            let table_name = schema
                .entities
                .get(&Symbol::from("JournalLine"))
                .map(|e| e.table_name.as_str())
                .unwrap_or("JournalLine");

            // Direct datastore access for generic filtering not supported fully via DataAccess::list yet (list returns Values)
            // But DataAccess::datastore() is available.
            let lines = data_access.datastore().find(table_name, filters).await.map_err(RuntimeError::WorkflowError)?;

            // 3. Create Reverse Header
            let mut new_entry = original.clone();
            let mut old_entry_number = "?".to_string();

            if let Some(obj) = new_entry.as_object_mut() {
                if let Some(num) = obj.get("entry_number").and_then(|v| v.as_str()) {
                    old_entry_number = num.to_string();
                }

                obj.remove("id");
                obj.insert("id".to_string(), json!(Uuid::new_v4().to_string()));
                obj.remove("entry_number");

                obj.insert("status".to_string(), json!("Draft"));
                obj.insert(
                    "description".to_string(),
                    json!(format!("Reversal of {}", old_entry_number)),
                );
                obj.insert("related_journal".to_string(), json!(id));
            }

            let new_id = data_access.create("JournalEntry", new_entry, ctx).await.map_err(RuntimeError::WorkflowError)?;

            // 4. Create Reverse Lines
            for line_arc in lines {
                let mut line = line_arc.as_ref().clone();
                if let Some(obj) = line.as_object_mut() {
                    obj.remove("id");
                    obj.insert("id".to_string(), json!(Uuid::new_v4().to_string()));
                    obj.insert("journal_entry".to_string(), json!(new_id));

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

                    obj.insert("debit".to_string(), json!(credit.to_string()));
                    obj.insert("credit".to_string(), json!(debit.to_string()));
                }
                data_access.create("JournalLine", line, ctx).await.map_err(RuntimeError::WorkflowError)?;
            }

            return Ok(true);
        }
        Ok(false)
    }
}
