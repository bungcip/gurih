use crate::datastore::DataStore;
use crate::errors::RuntimeError;
use async_trait::async_trait;
use chrono::NaiveDate;
use gurih_ir::{Expression, Schema, Symbol};
use serde_json::Value;
use std::sync::Arc;

#[async_trait]
pub trait WorkflowPlugin: Send + Sync {
    fn name(&self) -> &str;

    /// Checks a custom precondition.
    /// If the plugin recognizes the precondition name, it performs the check.
    /// If the check fails, it returns an Error.
    /// If the check passes or the plugin does not recognize the name, it returns Ok(()).
    async fn check_precondition(
        &self,
        name: &str,
        args: &[Expression],
        entity_data: &Value,
        schema: &Schema,
        datastore: Option<&Arc<dyn DataStore>>,
    ) -> Result<(), RuntimeError>;

    /// Applies a custom effect.
    /// Returns (updates, notifications, postings).
    /// If the plugin does not recognize the effect, it returns empty results.
    async fn apply_effect(
        &self,
        name: &str,
        args: &[Expression],
        schema: &Schema,
        entity_name: &str,
        entity_data: &Value,
    ) -> Result<(Value, Vec<String>, Vec<Symbol>), RuntimeError>;
}

pub struct FinancePlugin;

#[async_trait]
impl WorkflowPlugin for FinancePlugin {
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
                // Find fields that are arrays (composition/child tables)
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
                    // 1. Get transaction date
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
}
