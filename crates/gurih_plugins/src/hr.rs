use async_trait::async_trait;
use chrono::{Datelike, Local, NaiveDate};
use gurih_ir::{ActionStep, Expression, Schema, Symbol};
use gurih_runtime::context::RuntimeContext;
use gurih_runtime::datastore::DataStore;
use gurih_runtime::errors::RuntimeError;
use gurih_runtime::plugins::Plugin;
use gurih_runtime::traits::DataAccess;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;

pub struct HrPlugin;

#[async_trait]
impl Plugin for HrPlugin {
    fn name(&self) -> &str {
        "HrPlugin"
    }

    async fn check_precondition(
        &self,
        name: &str,
        args: &[Expression],
        kwargs: &HashMap<String, String>,
        entity_data: &Value,
        _schema: &Schema,
        datastore: Option<&Arc<dyn DataStore>>,
    ) -> Result<(), RuntimeError> {
        match name {
            "min_years_of_service" => {
                let years = if let Some(Expression::StringLiteral(s)) = args.first() {
                    s.parse::<f64>().unwrap_or(0.0)
                } else {
                    return Err(RuntimeError::ValidationError(
                        "min_years_of_service requires numeric argument".to_string(),
                    ));
                };
                let from_field = kwargs.get("from").cloned().unwrap_or_else(|| "join_date".to_string());

                // years_of_service(from_field) >= years
                let expr = Expression::BinaryOp {
                    left: Box::new(Expression::FunctionCall {
                        name: Symbol::from("years_of_service"),
                        args: vec![Expression::Field(Symbol::from(from_field))],
                    }),
                    op: gurih_ir::BinaryOperator::Gte,
                    right: Box::new(Expression::Literal(years)),
                };

                let result = gurih_runtime::evaluator::evaluate(&expr, entity_data, Some(_schema), datastore).await?;
                if result != Value::Bool(true) {
                    return Err(RuntimeError::ValidationError(format!(
                        "Minimum years of service not met: requires {}",
                        years
                    )));
                }
                Ok(())
            }
            "min_age" => {
                let age = if let Some(Expression::StringLiteral(s)) = args.first() {
                    s.parse::<f64>().unwrap_or(0.0)
                } else {
                    return Err(RuntimeError::ValidationError(
                        "min_age requires numeric argument".to_string(),
                    ));
                };
                let from_field = kwargs.get("from").cloned().unwrap_or_else(|| "birth_date".to_string());

                // age(from_field) >= age
                let expr = Expression::BinaryOp {
                    left: Box::new(Expression::FunctionCall {
                        name: Symbol::from("age"),
                        args: vec![Expression::Field(Symbol::from(from_field))],
                    }),
                    op: gurih_ir::BinaryOperator::Gte,
                    right: Box::new(Expression::Literal(age)),
                };

                let result = gurih_runtime::evaluator::evaluate(&expr, entity_data, Some(_schema), datastore).await?;
                if result != Value::Bool(true) {
                    return Err(RuntimeError::ValidationError(format!(
                        "Minimum age not met: requires {}",
                        age
                    )));
                }
                Ok(())
            }
            "valid_effective_date" => {
                let field = if let Some(Expression::StringLiteral(s)) = args.first() {
                    s
                } else {
                    return Err(RuntimeError::ValidationError(
                        "valid_effective_date requires field name".to_string(),
                    ));
                };

                // valid_date(field)
                let expr = Expression::FunctionCall {
                    name: Symbol::from("valid_date"),
                    args: vec![Expression::Field(Symbol::from(field))],
                };

                let result = gurih_runtime::evaluator::evaluate(&expr, entity_data, Some(_schema), datastore).await?;
                if result != Value::Bool(true) {
                    return Err(RuntimeError::ValidationError(format!(
                        "Invalid effective date for field {}",
                        field
                    )));
                }
                Ok(())
            }
            "check_kgb_eligibility" => {
                let field_name = if let Some(Expression::StringLiteral(s)) = args.first() {
                    s.as_str()
                } else {
                    "pegawai"
                };

                let pegawai_id = entity_data.get(field_name).and_then(|v| v.as_str()).ok_or_else(|| {
                    RuntimeError::ValidationError(format!("Field '{}' is missing or not a string", field_name))
                })?;

                let ds = datastore.ok_or_else(|| RuntimeError::InternalError("Datastore not available".to_string()))?;

                // Resolve table name from schema
                let table_name = _schema
                    .entities
                    .get(&Symbol::from("Pegawai"))
                    .map(|e| e.table_name.as_str())
                    .unwrap_or("Pegawai");

                // 1. Fetch Pegawai
                let pegawai_opt = ds
                    .get(table_name, pegawai_id)
                    .await
                    .map_err(RuntimeError::DataStoreError)?;

                let pegawai = if let Some(p) = pegawai_opt {
                    p
                } else {
                    return Err(RuntimeError::ValidationError(format!(
                        "Pegawai {} not found",
                        pegawai_id
                    )));
                };

                // 2. Check 2 Year Rule
                let tmt_golongan_str = pegawai.get("tmt_golongan").and_then(|v: &Value| v.as_str());
                let tmt_kgb_str = pegawai.get("tmt_kgb").and_then(|v: &Value| v.as_str());

                let tmt_golongan = tmt_golongan_str.and_then(|s| NaiveDate::parse_from_str(s, "%Y-%m-%d").ok());
                let tmt_kgb = tmt_kgb_str.and_then(|s| NaiveDate::parse_from_str(s, "%Y-%m-%d").ok());

                // Logic: 2 years from the LATEST of KP or KGB
                let last_date: NaiveDate = match (tmt_golongan, tmt_kgb) {
                    (Some(a), Some(b)) => {
                        if a > b {
                            a
                        } else {
                            b
                        }
                    }
                    (Some(a), None) => a,
                    (None, Some(b)) => b,
                    (None, None) => {
                        return Err(RuntimeError::ValidationError(
                            "Pegawai belum memiliki TMT Golongan atau TMT KGB".to_string(),
                        ));
                    }
                };

                let now = Local::now().date_naive();

                // Simple 2 year check
                let next_eligible_date = last_date.with_year(last_date.year() + 2).unwrap_or_else(|| {
                    // Handle leap year case (Feb 29 -> Feb 28)
                    NaiveDate::from_ymd_opt(last_date.year() + 2, 2, 28).unwrap()
                });

                if now < next_eligible_date {
                    return Err(RuntimeError::ValidationError(format!(
                        "Belum memenuhi syarat 2 tahun. Tanggal efektif berikutnya: {}",
                        next_eligible_date
                    )));
                }

                // 3. Check SKP Rule (Last 2 Years)
                let current_year = now.year();
                let year_1 = current_year - 1;
                let year_2 = current_year - 2;

                // Fetch all SKP for user
                let mut filters = HashMap::new();
                filters.insert("pegawai".to_string(), pegawai_id.to_string());

                let skp_table_name = _schema
                    .entities
                    .get(&Symbol::from("RiwayatSKP"))
                    .map(|e| e.table_name.as_str())
                    .unwrap_or("RiwayatSKP");

                let skps = ds
                    .find(skp_table_name, filters)
                    .await
                    .map_err(RuntimeError::DataStoreError)?;

                // Filter in memory for years
                let relevant_skps: Vec<&serde_json::Map<String, Value>> = skps
                    .iter()
                    .filter_map(|arc_val: &Arc<Value>| arc_val.as_object())
                    .filter(|obj: &&serde_json::Map<String, Value>| {
                        if let Some(val) = obj.get("tahun") {
                            // Handle number or string representation of year
                            let y = if let Some(n) = val.as_i64() {
                                n as i32
                            } else if let Some(s) = val.as_str() {
                                s.parse::<i32>().unwrap_or(0)
                            } else {
                                0
                            };
                            y == year_1 || y == year_2
                        } else {
                            false
                        }
                    })
                    .collect();

                // Ensure we have data for BOTH years
                let mut has_y1 = false;
                let mut has_y2 = false;

                for skp in &relevant_skps {
                    if let Some(val) = skp.get("tahun") {
                        let y = if let Some(n) = val.as_i64() {
                            n as i32
                        } else if let Some(s) = val.as_str() {
                            s.parse::<i32>().unwrap_or(0)
                        } else {
                            0
                        };
                        if y == year_1 {
                            has_y1 = true;
                        }
                        if y == year_2 {
                            has_y2 = true;
                        }
                    }
                }

                if !has_y1 || !has_y2 {
                    return Err(RuntimeError::ValidationError(format!(
                        "Data SKP tidak lengkap untuk tahun {} dan {}",
                        year_2, year_1
                    )));
                }

                for skp in relevant_skps {
                    let predikat = skp.get("predikat").and_then(|v: &Value| v.as_str()).unwrap_or("");
                    // "Baik" or "Sangat Baik"
                    if !predikat.contains("Baik") {
                        return Err(RuntimeError::ValidationError(
                            "Nilai SKP 2 tahun terakhir harus minimal 'Baik'".to_string(),
                        ));
                    }
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
        _kwargs: &HashMap<String, String>,
        _schema: &Schema,
        _datastore: Option<&Arc<dyn DataStore>>,
        _entity_name: &str,
        _entity_data: &Value,
    ) -> Result<(Value, Vec<String>, Vec<Symbol>), RuntimeError> {
        match name {
            "suspend_payroll" => {
                let suspend = if let Some(Expression::StringLiteral(s)) = args.first() {
                    s == "true"
                } else {
                    true
                };

                let mut updates = serde_json::Map::new();
                updates.insert("is_payroll_active".to_string(), Value::Bool(!suspend));

                Ok((Value::Object(updates), vec![], vec![]))
            }
            "update_rank_eligibility" => {
                let eligible = if let Some(Expression::StringLiteral(s)) = args.first() {
                    s == "true"
                } else {
                    true
                };

                let mut updates = serde_json::Map::new();
                updates.insert("rank_eligible".to_string(), Value::Bool(eligible));

                Ok((Value::Object(updates), vec![], vec![]))
            }
            _ => Ok((Value::Null, vec![], vec![])),
        }
    }

    async fn execute_action_step(
        &self,
        _step_name: &str,
        _step: &ActionStep,
        _params: &HashMap<String, String>,
        _data_access: &dyn DataAccess,
        _ctx: &RuntimeContext,
    ) -> Result<bool, RuntimeError> {
        Ok(false)
    }
}
