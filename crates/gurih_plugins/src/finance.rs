use crate::utils::{get_db_range_placeholders, parse_numeric_opt, resolve_param};
use async_trait::async_trait;
use chrono::NaiveDate;
use gurih_ir::{ActionStep, Expression, Schema, Symbol};
use gurih_runtime::context::RuntimeContext;
use gurih_runtime::datastore::DataStore;
use gurih_runtime::errors::RuntimeError;
use gurih_runtime::plugins::Plugin;
use gurih_runtime::store::validate_identifier;
use gurih_runtime::traits::DataAccess;
use serde_json::{Value, json};
use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;

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
        _kwargs: &HashMap<String, String>,
        entity_data: &Value,
        schema: &Schema,
        datastore: Option<&Arc<dyn DataStore>>,
    ) -> Result<(), RuntimeError> {
        match name {
            "balanced_transaction" => check_balanced_transaction(entity_data, schema, datastore).await,
            "valid_parties" => check_valid_parties(entity_data, schema, datastore).await,
            "period_open" => check_period_open(args, entity_data, schema, datastore).await,
            _ => Ok(()),
        }
    }

    async fn apply_effect(
        &self,
        name: &str,
        args: &[Expression],
        _kwargs: &HashMap<String, String>,
        _schema: &Schema,
        _entity_name: &str,
        _entity_data: &Value,
    ) -> Result<(Value, Vec<String>, Vec<Symbol>), RuntimeError> {
        if name == "post_journal"
            && let Some(Expression::StringLiteral(rule)) = args.first()
        {
            return Ok((Value::Null, vec![], vec![Symbol::from(rule.as_str())]));
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
            execute_reverse_journal(step, params, data_access, ctx).await
        } else if step_name == "finance:generate_closing_entry" {
            execute_generate_closing_entry(step, params, data_access, ctx).await
        } else {
            Ok(false)
        }
    }
}

async fn fetch_journal_lines(
    entity_data: &Value,
    schema: &Schema,
    datastore: Option<&Arc<dyn DataStore>>,
) -> Result<Vec<Value>, RuntimeError> {
    let mut lines = Vec::new();
    let mut found_lines_in_payload = false;

    // 1. Try to find lines in the payload
    if let Some(obj) = entity_data.as_object() {
        for (_key, val) in obj {
            if let Some(val_lines) = val.as_array() {
                for line in val_lines {
                    if let Some(line_obj) = line.as_object()
                        && (line_obj.contains_key("debit") || line_obj.contains_key("credit"))
                    {
                        lines.push(line.clone());
                        found_lines_in_payload = true;
                    }
                }
            }
        }
    }

    // 2. If not in payload, fetch from Datastore
    if !found_lines_in_payload
        && let Some(ds) = datastore
        && let Some(id) = entity_data.get("id").and_then(|v| v.as_str())
    {
        let table_name = schema
            .entities
            .get(&Symbol::from("JournalLine"))
            .map(|e| e.table_name.as_str())
            .unwrap_or("journal_line");

        let mut filters = HashMap::new();
        filters.insert("journal_entry".to_string(), id.to_string());

        let db_lines = ds
            .find(table_name, filters)
            .await
            .map_err(RuntimeError::WorkflowError)?;

        for line in db_lines {
            lines.push(line.as_ref().clone());
        }
    }

    Ok(lines)
}

async fn check_balanced_transaction(
    entity_data: &Value,
    schema: &Schema,
    datastore: Option<&Arc<dyn DataStore>>,
) -> Result<(), RuntimeError> {
    let lines = fetch_journal_lines(entity_data, schema, datastore).await?;

    let mut total_debit = 0.0;
    let mut total_credit = 0.0;

    for line in lines {
        if let Some(line_obj) = line.as_object() {
            total_debit += parse_numeric_opt(line_obj.get("debit"));
            total_credit += parse_numeric_opt(line_obj.get("credit"));
        }
    }

    let diff = (total_debit - total_credit).abs();
    if diff > 0.01 {
        return Err(RuntimeError::ValidationError(format!(
            "Transaction not balanced: Debit {:.2}, Credit {:.2} (Diff {:.2})",
            total_debit, total_credit, diff
        )));
    }

    Ok(())
}

async fn check_valid_parties(
    entity_data: &Value,
    schema: &Schema,
    datastore: Option<&Arc<dyn DataStore>>,
) -> Result<(), RuntimeError> {
    let ds = datastore
        .ok_or_else(|| RuntimeError::WorkflowError("Datastore not available for party validation".to_string()))?;

    let lines_to_check = fetch_journal_lines(entity_data, schema, Some(ds)).await?;

    // 2. Validate each line
    for line in lines_to_check {
        let account_id = line
            .get("account")
            .or_else(|| line.get("account_id")) // Support both forms
            .and_then(|v| v.as_str())
            .ok_or(RuntimeError::ValidationError(
                "Journal line missing account".to_string(),
            ))?;

        // Fetch Account
        let account_table = schema
            .entities
            .get(&Symbol::from("Account"))
            .map(|e| e.table_name.as_str())
            .unwrap_or("account");

        let account = ds
            .get(account_table, account_id)
            .await
            .map_err(RuntimeError::WorkflowError)?
            .ok_or(RuntimeError::ValidationError(format!(
                "Account not found: {}",
                account_id
            )))?;

        let requires_party = account.get("requires_party").and_then(|v| v.as_bool()).unwrap_or(false);

        let party_type = line.get("party_type").and_then(|v| v.as_str());
        let party_id = line.get("party_id").and_then(|v| v.as_str());

        if requires_party && (party_type.is_none() || party_id.is_none()) {
            let acc_code = account.get("code").and_then(|v| v.as_str()).unwrap_or("?");
            let acc_name = account.get("name").and_then(|v| v.as_str()).unwrap_or("?");
            return Err(RuntimeError::ValidationError(format!(
                "Account {} ({}) requires a Party (Customer/Vendor) to be specified.",
                acc_code, acc_name
            )));
        }

        // Verify Party Existence if specified
        if let (Some(pt), Some(pid)) = (party_type, party_id) {
            // Find entity table name
            let target_entity = schema.entities.get(&Symbol::from(pt));
            if let Some(entity_schema) = target_entity {
                let table = entity_schema.table_name.as_str();
                let exists = ds.get(table, pid).await.map_err(RuntimeError::WorkflowError)?.is_some();

                if !exists {
                    return Err(RuntimeError::ValidationError(format!(
                        "Referenced Party {} (Type: {}) does not exist.",
                        pid, pt
                    )));
                }
            } else {
                return Err(RuntimeError::ValidationError(format!("Unknown Party Type: {}", pt)));
            }
        }
    }

    Ok(())
}

async fn check_period_open(
    args: &[Expression],
    entity_data: &Value,
    schema: &Schema,
    datastore: Option<&Arc<dyn DataStore>>,
) -> Result<(), RuntimeError> {
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

            if let Err(e) = validate_identifier(target_entity) {
                return Err(RuntimeError::WorkflowError(format!(
                    "Invalid entity name for period check: {}",
                    e
                )));
            }

            let table_name = schema
                .entities
                .get(&Symbol::from(target_entity))
                .map(|e| e.table_name.as_str())
                .ok_or_else(|| {
                    RuntimeError::WorkflowError(format!(
                        "Entity '{}' not defined in schema for period check",
                        target_entity
                    ))
                })?;

            let db_type = schema
                .database
                .as_ref()
                .map(|d| d.db_type.clone())
                .unwrap_or(gurih_ir::DatabaseType::Sqlite);

            let (p_start, p_end) = get_db_range_placeholders(&db_type);

            let sql = format!(
                "SELECT id FROM {} WHERE status = 'Open' AND start_date <= {} AND end_date >= {}",
                table_name, p_start, p_end
            );

            let params = vec![Value::String(date_s.to_string()), Value::String(date_s.to_string())];

            let periods = ds
                .query_with_params(&sql, params)
                .await
                .map_err(RuntimeError::WorkflowError)?;
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

async fn execute_reverse_journal(
    step: &ActionStep,
    params: &HashMap<String, String>,
    data_access: &dyn DataAccess,
    ctx: &RuntimeContext,
) -> Result<bool, RuntimeError> {
    let id_raw = step.args.get("id").ok_or(RuntimeError::WorkflowError(
        "Missing 'id' argument for finance:reverse_journal".to_string(),
    ))?;
    let id = resolve_param(id_raw, params);

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

    let lines = data_access
        .datastore()
        .find(table_name, filters)
        .await
        .map_err(RuntimeError::WorkflowError)?;

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

    let new_id = data_access
        .create("JournalEntry", new_entry, ctx)
        .await
        .map_err(RuntimeError::WorkflowError)?;

    // 4. Create Reverse Lines
    for line_arc in lines {
        let mut line = line_arc.as_ref().clone();
        if let Some(obj) = line.as_object_mut() {
            obj.remove("id");
            obj.insert("id".to_string(), json!(Uuid::new_v4().to_string()));
            obj.insert("journal_entry".to_string(), json!(new_id));

            let debit = parse_numeric_opt(obj.get("debit"));
            let credit = parse_numeric_opt(obj.get("credit"));

            obj.insert("debit".to_string(), json!(credit.to_string()));
            obj.insert("credit".to_string(), json!(debit.to_string()));
        }
        data_access
            .create("JournalLine", line, ctx)
            .await
            .map_err(RuntimeError::WorkflowError)?;
    }

    Ok(true)
}

async fn execute_generate_closing_entry(
    step: &ActionStep,
    params: &HashMap<String, String>,
    data_access: &dyn DataAccess,
    ctx: &RuntimeContext,
) -> Result<bool, RuntimeError> {
    let period_id_raw = step
        .args
        .get("period_id")
        .ok_or(RuntimeError::WorkflowError("Missing 'period_id' argument".to_string()))?;
    let period_id = resolve_param(period_id_raw, params);

    // 1. Fetch Period
    let period_arc = data_access
        .read("AccountingPeriod", &period_id)
        .await
        .map_err(RuntimeError::WorkflowError)?
        .ok_or(RuntimeError::WorkflowError("AccountingPeriod not found".to_string()))?;

    let start_date = period_arc.get("start_date").and_then(|v| v.as_str()).unwrap_or("");
    let end_date = period_arc.get("end_date").and_then(|v| v.as_str()).unwrap_or("");
    let period_name = period_arc.get("name").and_then(|v| v.as_str()).unwrap_or("");

    // 2. Find Retained Earnings Account
    let mut filters = HashMap::new();
    filters.insert("name".to_string(), "Retained Earnings".to_string());

    let account_table = data_access
        .get_schema()
        .entities
        .get(&Symbol::from("Account"))
        .map(|e| e.table_name.as_str())
        .unwrap_or("account");

    let accounts = data_access
        .datastore()
        .find(account_table, filters)
        .await
        .map_err(RuntimeError::WorkflowError)?;
    let retained_earnings_id = accounts
        .first()
        .and_then(|a| a.get("id").and_then(|v| v.as_str()))
        .ok_or(RuntimeError::WorkflowError(
            "Retained Earnings account not found. Please add it to Chart of Accounts.".to_string(),
        ))?;

    // 3. Aggregate Revenue and Expense
    let db_type = data_access
        .get_schema()
        .database
        .as_ref()
        .map(|d| d.db_type.clone())
        .unwrap_or(gurih_ir::DatabaseType::Sqlite);

    let (p_start, p_end) = get_db_range_placeholders(&db_type);

    let schema = data_access.get_schema();
    let journal_line_table = schema
        .entities
        .get(&Symbol::from("JournalLine"))
        .map(|e| e.table_name.as_str())
        .unwrap_or("journal_line");
    let journal_entry_table = schema
        .entities
        .get(&Symbol::from("JournalEntry"))
        .map(|e| e.table_name.as_str())
        .unwrap_or("journal_entry");
    let account_table = schema
        .entities
        .get(&Symbol::from("Account"))
        .map(|e| e.table_name.as_str())
        .unwrap_or("account");

    let sql = format!(
        r#"
        SELECT
            jl.account as account_id,
            SUM(jl.debit) as total_debit,
            SUM(jl.credit) as total_credit,
            a.type as account_type
        FROM {} jl
        JOIN {} je ON jl.journal_entry = je.id
        JOIN {} a ON jl.account = a.id
        WHERE je.status = 'Posted'
          AND je.date >= {}
          AND je.date <= {}
          AND (a.type = 'Revenue' OR a.type = 'Expense')
        GROUP BY jl.account, a.type
    "#,
        journal_line_table, journal_entry_table, account_table, p_start, p_end
    );

    let params_vec = vec![
        Value::String(start_date.to_string()),
        Value::String(end_date.to_string()),
    ];

    let results = data_access
        .datastore()
        .query_with_params(&sql, params_vec)
        .await
        .map_err(RuntimeError::WorkflowError)?;

    let mut closing_lines = vec![];
    let mut total_retained_earnings_impact = 0.0; // Positive = Credit to RE (Profit), Negative = Debit to RE (Loss)

    for row in results {
        let account_id = row.get("account_id").and_then(|v| v.as_str()).unwrap_or("");
        let total_debit = parse_numeric_opt(row.get("total_debit"));
        let total_credit = parse_numeric_opt(row.get("total_credit"));

        // Round to 2 decimal places to avoid floating point issues
        let net = ((total_debit * 100.0).round() - (total_credit * 100.0).round()) / 100.0;

        if net.abs() < 0.01 {
            continue;
        }

        let mut line = serde_json::Map::new();
        line.insert("account".to_string(), Value::String(account_id.to_string()));

        if net > 0.0 {
            // Debit Balance (Expense) -> Credit it
            line.insert("credit".to_string(), Value::String(format!("{:.2}", net)));
            line.insert("debit".to_string(), Value::String("0.00".to_string()));
            // Effect on RE: Expense reduces Equity (Debit RE).
            // We Credited Expense, so we Debit RE.
            // Impact tracks Credit side. So Debit = Negative Impact.
            total_retained_earnings_impact -= net;
        } else {
            // Credit Balance (Revenue) -> Debit it
            let amount = net.abs();
            line.insert("debit".to_string(), Value::String(format!("{:.2}", amount)));
            line.insert("credit".to_string(), Value::String("0.00".to_string()));
            // Effect on RE: Revenue increases Equity (Credit RE).
            // We Debited Revenue, so we Credit RE.
            // Impact tracks Credit side. So Credit = Positive Impact.
            total_retained_earnings_impact += amount;
        }
        closing_lines.push(Value::Object(line));
    }

    if closing_lines.is_empty() {
        return Ok(true);
    }

    // Add Plug Line for Retained Earnings
    let mut plug_line = serde_json::Map::new();
    plug_line.insert("account".to_string(), Value::String(retained_earnings_id.to_string()));

    // Round impact
    let impact = (total_retained_earnings_impact * 100.0).round() / 100.0;

    if impact > 0.0 {
        // Profit -> Credit RE
        plug_line.insert("credit".to_string(), Value::String(format!("{:.2}", impact)));
        plug_line.insert("debit".to_string(), Value::String("0.00".to_string()));
    } else {
        // Loss -> Debit RE
        plug_line.insert("debit".to_string(), Value::String(format!("{:.2}", impact.abs())));
        plug_line.insert("credit".to_string(), Value::String("0.00".to_string()));
    }
    closing_lines.push(Value::Object(plug_line));

    // Create Journal Entry
    let mut journal = serde_json::Map::new();
    journal.insert(
        "description".to_string(),
        Value::String(format!("Closing Entry for {}", period_name)),
    );
    journal.insert("date".to_string(), Value::String(end_date.to_string()));
    journal.insert("status".to_string(), Value::String("Draft".to_string()));

    let new_journal_id = data_access
        .create("JournalEntry", Value::Object(journal), ctx)
        .await
        .map_err(RuntimeError::WorkflowError)?;

    // Create Journal Lines
    for mut line_val in closing_lines {
        if let Some(obj) = line_val.as_object_mut() {
            obj.insert("journal_entry".to_string(), Value::String(new_journal_id.clone()));
        }
        data_access
            .create("JournalLine", line_val, ctx)
            .await
            .map_err(RuntimeError::WorkflowError)?;
    }

    Ok(true)
}
