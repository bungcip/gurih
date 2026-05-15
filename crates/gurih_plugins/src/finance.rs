use async_trait::async_trait;
use chrono::{Local, NaiveDate};
use futures::future::join_all;
use gurih_ir::utils::{get_db_range_placeholders, parse_numeric_opt, resolve_param};
use gurih_ir::{ActionStep, Expression, Schema, Symbol};
use gurih_runtime::context::RuntimeContext;
use gurih_runtime::datastore::DataStore;
use gurih_runtime::errors::RuntimeError;
use gurih_runtime::plugins::Plugin;
use gurih_runtime::store::validate_identifier;
use gurih_runtime::traits::DataAccess;
use rust_decimal::Decimal;
use serde_json::{Value, json};
use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;

pub struct FinancePlugin;

fn parse_decimal_opt(v: Option<&Value>) -> Result<Decimal, RuntimeError> {
    match v {
        Some(val) => {
            if let Some(s) = val.as_str() {
                s.parse()
                    .map_err(|_| RuntimeError::ValidationError(format!("Invalid decimal amount: '{}'", s)))
            } else if let Some(n) = val.as_f64() {
                // Best effort conversion from float
                Decimal::from_f64_retain(n)
                    .ok_or_else(|| RuntimeError::ValidationError(format!("Invalid float amount: '{}'", n)))
            } else if val.is_null() {
                Ok(Decimal::ZERO)
            } else {
                Err(RuntimeError::ValidationError(format!(
                    "Invalid decimal amount type: '{}'",
                    val
                )))
            }
        }
        None => Ok(Decimal::ZERO),
    }
}

fn get_validated_table_name<'a>(
    schema: &'a Schema,
    entity_name: &str,
    default_table_name: &'a str,
) -> Result<&'a str, RuntimeError> {
    let table_name = schema
        .entities
        .get(&Symbol::from(entity_name))
        .map_or(default_table_name, |e| e.table_name.as_str());
    validate_identifier(table_name)
        .map_err(|e| RuntimeError::WorkflowError(format!("Invalid table_name for {}: {}", entity_name, e)))?;
    Ok(table_name)
}

fn get_validated_table_name_strict<'a>(schema: &'a Schema, entity_name: &str) -> Result<&'a str, RuntimeError> {
    let table_name = schema
        .entities
        .get(&Symbol::from(entity_name))
        .map(|e| e.table_name.as_str())
        .ok_or_else(|| RuntimeError::WorkflowError(format!("Entity '{}' not defined in schema", entity_name)))?;
    validate_identifier(table_name)
        .map_err(|e| RuntimeError::WorkflowError(format!("Invalid table_name for {}: {}", entity_name, e)))?;
    Ok(table_name)
}

fn get_db_type(schema: &Schema) -> gurih_ir::DatabaseType {
    schema
        .database
        .as_ref()
        .map_or(gurih_ir::DatabaseType::Sqlite, |d| d.db_type.clone())
}

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
            "no_period_overlap" => check_period_overlap(entity_data, schema, datastore).await,
            _ => Ok(()),
        }
    }

    async fn apply_effect(
        &self,
        name: &str,
        args: &[Expression],
        _kwargs: &HashMap<String, String>,
        schema: &Schema,
        datastore: Option<&Arc<dyn DataStore>>,
        _entity_name: &str,
        entity_data: &Value,
    ) -> Result<(Value, Vec<String>, Vec<Symbol>), RuntimeError> {
        match name {
            "post_journal" => {
                if let Some(Expression::StringLiteral(rule)) = args.first() {
                    return Ok((Value::Null, vec![], vec![Symbol::from(rule.as_str())]));
                }
            }
            "snapshot_parties" => {
                execute_snapshot_parties(entity_data, schema, datastore).await?;
            }
            "init_line_status" => {
                execute_init_line_status(entity_data, schema, datastore).await?;
            }
            _ => {}
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
        match step_name {
            "finance:reverse_journal" => execute_reverse_journal(step, params, data_access, ctx).await,
            "finance:generate_closing_entry" => execute_generate_closing_entry(step, params, data_access, ctx).await,
            "finance:reconcile_entries" => execute_reconcile_entries(step, params, data_access, ctx).await,
            _ => Ok(false),
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
        // Prioritize standard standard "lines" key
        if let Some(val_lines) = obj.get("lines").and_then(|v| v.as_array()) {
            for line in val_lines {
                if let Some(line_obj) = line.as_object()
                    && (line_obj.contains_key("debit") || line_obj.contains_key("credit"))
                {
                    lines.push(line.clone());
                    found_lines_in_payload = true;
                }
            }
        }

        // Fallback to searching other keys if "lines" was empty or not found
        if !found_lines_in_payload {
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
                    if found_lines_in_payload {
                        break;
                    }
                }
            }
        }
    }

    // 2. If not in payload, fetch from Datastore
    if !found_lines_in_payload && let (Some(ds), Some(id)) = (datastore, entity_data.get("id").and_then(|v| v.as_str()))
    {
        let table_name = get_validated_table_name(schema, "JournalLine", "journal_line")?;

        let mut filters = HashMap::new();
        filters.insert("journal_entry".to_string(), id.to_string());

        let db_lines = ds
            .find(table_name, filters)
            .await
            .map_err(RuntimeError::WorkflowError)?;

        lines.extend(db_lines.into_iter().map(Arc::unwrap_or_clone));
    }

    Ok(lines)
}

async fn check_balanced_transaction(
    entity_data: &Value,
    schema: &Schema,
    datastore: Option<&Arc<dyn DataStore>>,
) -> Result<(), RuntimeError> {
    let lines = fetch_journal_lines(entity_data, schema, datastore).await?;

    let entry_id = entity_data
        .get("id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| RuntimeError::ValidationError("Transaction missing id".to_string()))?;

    if lines.is_empty() {
        return Err(RuntimeError::ValidationError(format!(
            "Transaction {} must have at least one line",
            entry_id
        )));
    }

    let mut total_debit = Decimal::ZERO;
    let mut total_credit = Decimal::ZERO;

    for line in lines {
        if let Some(line_obj) = line.as_object() {
            total_debit += parse_decimal_opt(line_obj.get("debit"))?;
            total_credit += parse_decimal_opt(line_obj.get("credit"))?;
        }
    }

    if total_debit.is_zero() && total_credit.is_zero() {
        return Err(RuntimeError::ValidationError(format!(
            "Transaction {} cannot have a zero balance",
            entry_id
        )));
    }

    let diff = (total_debit - total_credit).abs();
    if !diff.is_zero() {
        return Err(RuntimeError::ValidationError(format!(
            "Transaction not balanced for Entry {}: Debit {}, Credit {} (Diff {})",
            entry_id, total_debit, total_credit, diff
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

    // --- Optimization: Batch Fetch Accounts & Parties ---
    // Instead of N+1 lookups, we try to fetch all referenced accounts and parties in batch.
    // If the datastore (e.g. MemoryDataStore) doesn't support raw SQL queries, we fallback to iterative lookups.

    let mut account_ids = std::collections::HashSet::new();
    let mut parties_to_check: HashMap<String, std::collections::HashSet<String>> = HashMap::new();

    for line in &lines_to_check {
        if let Some(id) = line
            .get("account")
            .or_else(|| line.get("account_id"))
            .and_then(|v| v.as_str())
        {
            account_ids.insert(id.to_string());
        }

        if let (Some(pt), Some(pid)) = (
            line.get("party_type").and_then(|v| v.as_str()),
            line.get("party_id").and_then(|v| v.as_str()),
        ) {
            parties_to_check
                .entry(pt.to_string())
                .or_default()
                .insert(pid.to_string());
        }
    }

    let db_type = get_db_type(schema);

    // Cache: Account ID -> Account Data
    let mut accounts_cache: HashMap<String, Arc<Value>> = HashMap::new();
    // Cache: (PartyType, PartyID) -> Exists (bool)
    let mut party_existence_cache: HashMap<(String, String), bool> = HashMap::new();

    // 1. Batch Fetch Accounts
    if !account_ids.is_empty() {
        let account_table = get_validated_table_name(schema, "Account", "account")?;

        let ids: Vec<String> = account_ids.into_iter().collect();
        let placeholders = (1..=ids.len())
            .map(|i| gurih_ir::utils::get_db_placeholder(&db_type, i))
            .collect::<Vec<_>>()
            .join(", ");

        let sql = format!("SELECT * FROM {} WHERE id IN ({})", account_table, placeholders);
        let params: Vec<Value> = ids.iter().map(|s| Value::String(s.clone())).collect();

        // Try query_with_params
        match ds.query_with_params(&sql, params).await {
            Ok(results) => {
                for acc in results {
                    if let Some(id) = acc.get("id").and_then(|v| v.as_str()) {
                        accounts_cache.insert(id.to_string(), acc.clone());
                    }
                }
            }
            Err(e) if e.contains("Raw SQL query not supported") => {
                // Fallback: Fetch one by one (MemoryDataStore) in parallel
                let futs = ids.iter().map(|id| ds.get(account_table, id));
                let results = join_all(futs).await;
                for (id, res) in ids.into_iter().zip(results) {
                    if let Some(acc) = res.map_err(RuntimeError::WorkflowError)? {
                        accounts_cache.insert(id, acc);
                    }
                }
            }
            Err(e) => return Err(RuntimeError::WorkflowError(e)),
        }
    }

    // 2. Batch Fetch Parties
    for (pt, pids) in parties_to_check {
        let target_entity = schema.entities.get(&Symbol::from(pt.as_str()));
        if target_entity.is_some() {
            let fallback_table = pt.to_lowercase();
            let table = get_validated_table_name(schema, pt.as_str(), fallback_table.as_str())?;
            let ids: Vec<String> = pids.into_iter().collect();
            let placeholders = (1..=ids.len())
                .map(|i| gurih_ir::utils::get_db_placeholder(&db_type, i))
                .collect::<Vec<_>>()
                .join(", ");

            let sql = format!("SELECT id FROM {} WHERE id IN ({})", table, placeholders);
            let params: Vec<Value> = ids.iter().map(|s| Value::String(s.clone())).collect();

            match ds.query_with_params(&sql, params).await {
                Ok(results) => {
                    // Mark found
                    for row in results {
                        if let Some(id) = row.get("id").and_then(|v| v.as_str()) {
                            party_existence_cache.insert((pt.clone(), id.to_string()), true);
                        }
                    }
                    // Mark not found (implicitly handled by lookup failure in cache, but for fallback consistency we just fill cache)
                }
                Err(e) if e.contains("Raw SQL query not supported") => {
                    // Fallback in parallel
                    let futs = ids.iter().map(|id| ds.get(table, id));
                    let results = join_all(futs).await;
                    for (id, res) in ids.into_iter().zip(results) {
                        let exists = res.map_err(RuntimeError::WorkflowError)?.is_some();
                        if exists {
                            party_existence_cache.insert((pt.clone(), id), true);
                        }
                    }
                }
                Err(e) => return Err(RuntimeError::WorkflowError(e)),
            }
        } else {
            // Unknown Party Type handled in loop
        }
    }

    // 2. Validate each line using Cache
    let entry_id = entity_data
        .get("id")
        .and_then(|v| v.as_str())
        .unwrap_or("Unknown");

    for line in lines_to_check {
        let account_id = line
            .get("account")
            .or_else(|| line.get("account_id")) // Support both forms
            .and_then(|v| v.as_str())
            .ok_or_else(|| RuntimeError::ValidationError(format!("Journal line missing account in Entry {}", entry_id)))?;

        let account = accounts_cache
            .get(account_id)
            .ok_or_else(|| RuntimeError::ValidationError(format!("Account not found: {} in Entry {}", account_id, entry_id)))?;

        let requires_party = account
            .get("requires_party")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(false);

        let party_type = line.get("party_type").and_then(|v| v.as_str());
        let party_id = line.get("party_id").and_then(|v| v.as_str());

        if requires_party && (party_type.is_none() || party_id.is_none()) {
            let acc_code = account
                .get("code")
                .and_then(|v| v.as_str())
                .ok_or_else(|| RuntimeError::ValidationError(format!("Account missing code in Entry {}", entry_id)))?;
            let acc_name = account
                .get("name")
                .and_then(|v| v.as_str())
                .ok_or_else(|| RuntimeError::ValidationError(format!("Account missing name in Entry {}", entry_id)))?;
            return Err(RuntimeError::ValidationError(format!(
                "Account {} ({}) requires a Party (Customer/Vendor) to be specified in Entry {}.",
                acc_code, acc_name, entry_id
            )));
        }

        // Verify Party Existence if specified
        if let (Some(pt), Some(pid)) = (party_type, party_id) {
            let target_entity = schema.entities.get(&Symbol::from(pt));
            if target_entity.is_some() {
                // Check cache
                if !party_existence_cache.contains_key(&(pt.to_string(), pid.to_string())) {
                    return Err(RuntimeError::ValidationError(format!(
                        "Referenced Party {} (Type: {}) does not exist in Entry {}.",
                        pid, pt, entry_id
                    )));
                }
            } else {
                return Err(RuntimeError::ValidationError(format!("Unknown Party Type: {} in Entry {}", pt, entry_id)));
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

            let table_name = get_validated_table_name_strict(schema, target_entity).map_err(|e| {
                if let RuntimeError::WorkflowError(ref msg) = e {
                    if msg.starts_with("Invalid table_name for") {
                        return RuntimeError::WorkflowError(format!("Invalid entity name for period check: {}", msg));
                    }
                    if msg.starts_with("Entity '") && msg.ends_with("' not defined in schema") {
                        return RuntimeError::WorkflowError(format!(
                            "Entity '{}' not defined in schema for period check",
                            target_entity
                        ));
                    }
                }
                e
            })?;

            let db_type = get_db_type(schema);

            let (p_start, p_end) = get_db_range_placeholders(&db_type);

            let sql = format!(
                "SELECT id FROM {} WHERE status = 'Open' AND start_date <= {} AND end_date >= {}",
                table_name, p_start, p_end
            );

            let params = vec![Value::String(date_s.to_string()), Value::String(date_s.to_string())];

            let is_empty = match ds.query_with_params(&sql, params.clone()).await {
                Ok(periods) => periods.is_empty(),
                Err(e) if e.contains("Raw SQL query not supported") => {
                    // Fallback for MemoryDataStore
                    let mut filters = HashMap::new();
                    filters.insert("status".to_string(), "Open".to_string());
                    let all_periods = ds
                        .find(table_name, filters)
                        .await
                        .map_err(RuntimeError::WorkflowError)?;
                    let mut found = false;
                    for p in all_periods {
                        if let (Some(start), Some(end)) = (
                            p.get("start_date").and_then(|v| v.as_str()),
                            p.get("end_date").and_then(|v| v.as_str()),
                        ) && start <= date_s
                            && end >= date_s
                        {
                            found = true;
                            break;
                        }
                    }
                    !found
                }
                Err(e) => return Err(RuntimeError::WorkflowError(e)),
            };

            if is_empty {
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

async fn execute_init_line_status(
    entity_data: &Value,
    schema: &Schema,
    datastore: Option<&Arc<dyn DataStore>>,
) -> Result<(), RuntimeError> {
    if let (Some(ds), Some(journal_id)) = (datastore, entity_data.get("id").and_then(|v| v.as_str())) {
        // 1. Fetch Journal Lines
        let table_name = get_validated_table_name(schema, "JournalLine", "journal_line")?;

        let mut filters = HashMap::new();
        filters.insert("journal_entry".to_string(), journal_id.to_string());

        let lines = ds
            .find(table_name, filters)
            .await
            .map_err(RuntimeError::WorkflowError)?;

        let mut status_records = Vec::with_capacity(lines.len());

        for line_arc in lines {
            let line = line_arc.as_ref();
            if let Some(lid) = line.get("id").and_then(|v| v.as_str()) {
                let debit = parse_decimal_opt(line.get("debit"))?;
                let credit = parse_decimal_opt(line.get("credit"))?;

                // Residual is the amount (debit or credit)
                let amount = (debit - credit).abs();

                let mut status = serde_json::Map::new();
                status.insert("journal_line".to_string(), Value::String(lid.to_string()));
                status.insert("amount_residual".to_string(), Value::String(amount.to_string()));
                status.insert("is_fully_reconciled".to_string(), Value::Bool(amount.is_zero()));

                // Ensure ID is generated
                status.insert("id".to_string(), Value::String(Uuid::new_v4().to_string()));

                status_records.push(Value::Object(status));
            }
        }

        if !status_records.is_empty() {
            let status_table = get_validated_table_name(schema, "JournalLineStatus", "journal_line_status")?;

            ds.insert_many(status_table, status_records)
                .await
                .map_err(RuntimeError::WorkflowError)?;
        }
    }
    Ok(())
}

async fn execute_reconcile_entries(
    step: &ActionStep,
    params: &HashMap<String, String>,
    data_access: &dyn DataAccess,
    ctx: &RuntimeContext,
) -> Result<bool, RuntimeError> {
    let debit_line_id = resolve_param(
        step.args
            .get("debit_line_id")
            .ok_or_else(|| RuntimeError::WorkflowError("Missing debit_line_id".to_string()))?,
        params,
    );
    let credit_line_id = resolve_param(
        step.args
            .get("credit_line_id")
            .ok_or_else(|| RuntimeError::WorkflowError("Missing credit_line_id".to_string()))?,
        params,
    );
    let amount_str = resolve_param(
        step.args
            .get("amount")
            .ok_or_else(|| RuntimeError::WorkflowError("Missing amount".to_string()))?,
        params,
    );
    let amount = amount_str
        .parse::<Decimal>()
        .map_err(|_| RuntimeError::ValidationError(format!("Invalid reconciliation amount: '{}'", amount_str)))?;

    if amount <= Decimal::ZERO {
        return Err(RuntimeError::ValidationError(
            "Reconciliation amount must be positive".to_string(),
        ));
    }

    // 1. Fetch Lines
    let line_entity = "JournalLine";
    let debit_line_arc = data_access
        .read(line_entity, &debit_line_id, ctx)
        .await
        .map_err(RuntimeError::WorkflowError)?
        .ok_or_else(|| RuntimeError::ValidationError("Debit line not found".to_string()))?;
    let credit_line_arc = data_access
        .read(line_entity, &credit_line_id, ctx)
        .await
        .map_err(RuntimeError::WorkflowError)?
        .ok_or_else(|| RuntimeError::ValidationError("Credit line not found".to_string()))?;

    let debit_line = debit_line_arc.as_ref();
    let credit_line = credit_line_arc.as_ref();

    // 2. Validate Match (Account & Party)
    let d_acc = debit_line
        .get("account")
        .or_else(|| debit_line.get("account_id"))
        .and_then(|v| v.as_str())
        .ok_or_else(|| RuntimeError::ValidationError("Debit line missing account".to_string()))?;

    let c_acc = credit_line
        .get("account")
        .or_else(|| credit_line.get("account_id"))
        .and_then(|v| v.as_str())
        .ok_or_else(|| RuntimeError::ValidationError("Credit line missing account".to_string()))?;

    if d_acc != c_acc {
        return Err(RuntimeError::ValidationError(
            "Cannot reconcile lines from different accounts".to_string(),
        ));
    }

    let d_party = debit_line
        .get("party_id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| RuntimeError::ValidationError("Debit line missing party_id".to_string()))?;

    let c_party = credit_line
        .get("party_id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| RuntimeError::ValidationError("Credit line missing party_id".to_string()))?;

    if d_party != c_party {
        return Err(RuntimeError::ValidationError(
            "Cannot reconcile lines from different parties".to_string(),
        ));
    }

    // 3. Fetch Statuses
    let status_table = get_validated_table_name(data_access.get_schema(), "JournalLineStatus", "journal_line_status")?;
    let ds = data_access.datastore();

    let mut d_filters = HashMap::new();
    d_filters.insert("journal_line".to_string(), debit_line_id.clone());
    let d_status_arc = ds
        .find_first(status_table, d_filters)
        .await
        .map_err(RuntimeError::WorkflowError)?
        .ok_or_else(|| RuntimeError::ValidationError("Debit line status not found".to_string()))?;

    let mut c_filters = HashMap::new();
    c_filters.insert("journal_line".to_string(), credit_line_id.clone());
    let c_status_arc = ds
        .find_first(status_table, c_filters)
        .await
        .map_err(RuntimeError::WorkflowError)?
        .ok_or_else(|| RuntimeError::ValidationError("Credit line status not found".to_string()))?;

    let d_residual = parse_decimal_opt(d_status_arc.get("amount_residual"))?;
    let c_residual = parse_decimal_opt(c_status_arc.get("amount_residual"))?;

    if amount > d_residual {
        return Err(RuntimeError::ValidationError(format!(
            "Amount {} exceeds debit residual {}",
            amount, d_residual
        )));
    }
    if amount > c_residual {
        return Err(RuntimeError::ValidationError(format!(
            "Amount {} exceeds credit residual {}",
            amount, c_residual
        )));
    }

    // 4. Update Statuses
    let update_status = |current_res: Decimal, sub: Decimal| {
        let new_res = current_res - sub;
        let mut map = serde_json::Map::new();
        map.insert("amount_residual".to_string(), Value::String(new_res.to_string()));
        if new_res.is_zero() {
            map.insert("is_fully_reconciled".to_string(), Value::Bool(true));
        }
        map
    };

    let d_update = update_status(d_residual, amount);
    let d_status_id = d_status_arc
        .get("id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| RuntimeError::ValidationError("Debit line status missing id".to_string()))?;
    ds.update(status_table, d_status_id, Value::Object(d_update))
        .await
        .map_err(RuntimeError::WorkflowError)?;

    let c_update = update_status(c_residual, amount);
    let c_status_id = c_status_arc
        .get("id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| RuntimeError::ValidationError("Credit line status missing id".to_string()))?;
    ds.update(status_table, c_status_id, Value::Object(c_update))
        .await
        .map_err(RuntimeError::WorkflowError)?;

    // 5. Create Reconciliation Record
    let mut rec = serde_json::Map::new();
    rec.insert("debit_line".to_string(), Value::String(debit_line_id));
    rec.insert("credit_line".to_string(), Value::String(credit_line_id));
    rec.insert("amount".to_string(), Value::String(amount.to_string()));
    rec.insert(
        "date".to_string(),
        Value::String(Local::now().format("%Y-%m-%d").to_string()),
    );

    data_access
        .create("Reconciliation", Value::Object(rec), ctx)
        .await
        .map_err(RuntimeError::WorkflowError)?;

    Ok(true)
}

async fn check_period_overlap(
    entity_data: &Value,
    schema: &Schema,
    datastore: Option<&Arc<dyn DataStore>>,
) -> Result<(), RuntimeError> {
    let ds = datastore.ok_or_else(|| RuntimeError::WorkflowError("Datastore not available".to_string()))?;

    // 1. Get dates from current entity
    let start_date_s = entity_data
        .get("start_date")
        .and_then(|v| v.as_str())
        .ok_or_else(|| RuntimeError::ValidationError("Missing start_date".to_string()))?;
    let end_date_s = entity_data
        .get("end_date")
        .and_then(|v| v.as_str())
        .ok_or_else(|| RuntimeError::ValidationError("Missing end_date".to_string()))?;

    let start_date = NaiveDate::parse_from_str(start_date_s, "%Y-%m-%d")
        .map_err(|_| RuntimeError::ValidationError(format!("Invalid start_date format: '{}'", start_date_s)))?;
    let end_date = NaiveDate::parse_from_str(end_date_s, "%Y-%m-%d")
        .map_err(|_| RuntimeError::ValidationError(format!("Invalid end_date format: '{}'", end_date_s)))?;

    if start_date > end_date {
        return Err(RuntimeError::ValidationError(
            "Start date must be before or equal to end date".to_string(),
        ));
    }

    let current_id = entity_data.get("id").and_then(|v| v.as_str());

    // 2. Fetch all periods
    let table_name = get_validated_table_name(schema, "AccountingPeriod", "accounting_period")?;

    let all_periods = ds
        .list(table_name, None, None)
        .await
        .map_err(RuntimeError::WorkflowError)?;

    // 3. Check Overlap
    for period in all_periods {
        // Skip self
        let pid_opt = period.get("id").and_then(|v| v.as_str());
        let is_same = match (pid_opt, current_id) {
            (Some(pid), Some(cid)) => pid == cid,
            _ => false,
        };
        if is_same {
            continue;
        }

        // Skip Draft
        let is_draft = match period.get("status").and_then(|v| v.as_str()) {
            Some(s) => s == "Draft",
            _ => false,
        };
        if is_draft {
            continue;
        }

        let p_start_s = period
            .get("start_date")
            .and_then(|v| v.as_str())
            .ok_or_else(|| RuntimeError::ValidationError("Existing period missing start_date".to_string()))?;
        let p_end_s = period
            .get("end_date")
            .and_then(|v| v.as_str())
            .ok_or_else(|| RuntimeError::ValidationError("Existing period missing end_date".to_string()))?;

        let p_start = NaiveDate::parse_from_str(p_start_s, "%Y-%m-%d").map_err(|_| {
            RuntimeError::ValidationError(format!("Invalid start_date format in existing period: '{}'", p_start_s))
        })?;
        let p_end = NaiveDate::parse_from_str(p_end_s, "%Y-%m-%d").map_err(|_| {
            RuntimeError::ValidationError(format!("Invalid end_date format in existing period: '{}'", p_end_s))
        })?;

        // Overlap logic: start1 <= end2 AND end1 >= start2
        if start_date <= p_end && end_date >= p_start {
            let name = period
                .get("name")
                .and_then(|v| v.as_str())
                .ok_or_else(|| RuntimeError::ValidationError("Existing period missing name".to_string()))?;
            return Err(RuntimeError::ValidationError(format!(
                "Period overlaps with existing period '{}'",
                name
            )));
        }
    }

    Ok(())
}

async fn execute_reverse_journal(
    step: &ActionStep,
    params: &HashMap<String, String>,
    data_access: &dyn DataAccess,
    ctx: &RuntimeContext,
) -> Result<bool, RuntimeError> {
    let id_raw = step
        .args
        .get("id")
        .ok_or_else(|| RuntimeError::WorkflowError("Missing 'id' argument for finance:reverse_journal".to_string()))?;
    let id = resolve_param(id_raw, params);

    // 1. Read Original
    let original_arc = data_access
        .read("JournalEntry", &id, ctx)
        .await
        .map_err(RuntimeError::WorkflowError)?
        .ok_or_else(|| RuntimeError::WorkflowError("JournalEntry not found".to_string()))?;
    let original = original_arc.as_ref();

    // 2. Read Lines
    let mut filters = HashMap::new();
    filters.insert("journal_entry".to_string(), id.clone());

    let schema = data_access.get_schema();
    let table_name = get_validated_table_name(schema, "JournalLine", "journal_line")?;

    let lines = data_access
        .datastore()
        .find(table_name, filters)
        .await
        .map_err(RuntimeError::WorkflowError)?;

    // 3. Create Reverse Header
    let mut new_entry = original.clone();

    if let Some(obj) = new_entry.as_object_mut() {
        let old_entry_number = obj
            .get("entry_number")
            .and_then(|v| v.as_str())
            .ok_or_else(|| RuntimeError::ValidationError("Original journal entry missing entry_number".to_string()))?
            .to_string();

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
    let mut reverse_lines = Vec::with_capacity(lines.len());
    for line_arc in lines {
        let mut line = Arc::unwrap_or_clone(line_arc);
        if let Some(obj) = line.as_object_mut() {
            obj.remove("id");
            obj.insert("id".to_string(), json!(Uuid::new_v4().to_string()));
            obj.insert("journal_entry".to_string(), json!(new_id));

            // Use string parsing to preserve precision if strings are available
            let debit_val = obj.get("debit");
            let credit_val = obj.get("credit");

            let debit_str = if let Some(s) = debit_val.and_then(|v| v.as_str()) {
                s.to_string()
            } else {
                parse_numeric_opt(debit_val).to_string()
            };

            let credit_str = if let Some(s) = credit_val.and_then(|v| v.as_str()) {
                s.to_string()
            } else {
                parse_numeric_opt(credit_val).to_string()
            };

            obj.insert("debit".to_string(), json!(credit_str));
            obj.insert("credit".to_string(), json!(debit_str));
        }
        reverse_lines.push(line);
    }

    data_access
        .create_many("JournalLine", reverse_lines, ctx)
        .await
        .map_err(RuntimeError::WorkflowError)?;

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
        .ok_or_else(|| RuntimeError::WorkflowError("Missing 'period_id' argument".to_string()))?;
    let period_id = resolve_param(period_id_raw, params);

    // 1. Fetch Period
    let period_arc = data_access
        .read("AccountingPeriod", &period_id, ctx)
        .await
        .map_err(RuntimeError::WorkflowError)?
        .ok_or_else(|| RuntimeError::WorkflowError("AccountingPeriod not found".to_string()))?;

    let start_date = period_arc
        .get("start_date")
        .and_then(|v| v.as_str())
        .ok_or_else(|| RuntimeError::ValidationError("Missing start_date in period".to_string()))?;
    let end_date = period_arc
        .get("end_date")
        .and_then(|v| v.as_str())
        .ok_or_else(|| RuntimeError::ValidationError("Missing end_date in period".to_string()))?;
    let period_name = period_arc
        .get("name")
        .and_then(|v| v.as_str())
        .ok_or_else(|| RuntimeError::ValidationError("Missing name in period".to_string()))?;

    // 2. Find Retained Earnings Account
    let mut filters = HashMap::new();
    filters.insert("system_tag".to_string(), "retained_earnings".to_string());

    let account_table = get_validated_table_name(data_access.get_schema(), "Account", "account")?;

    let accounts = data_access
        .datastore()
        .find(account_table, filters)
        .await
        .map_err(RuntimeError::WorkflowError)?;
    let retained_earnings_id = accounts
        .first()
        .and_then(|a| a.get("id").and_then(|v| v.as_str()))
        .ok_or_else(|| {
            RuntimeError::WorkflowError(
            "Retained Earnings account not found. Please ensure an account with system_tag='retained_earnings' exists."
                .to_string(),
        )
        })?;

    // 3. Aggregate Revenue and Expense
    // We fetch raw lines instead of SUM() to ensure decimal precision
    let db_type = get_db_type(data_access.get_schema());

    let (p_start, p_end) = get_db_range_placeholders(&db_type);

    let schema = data_access.get_schema();
    let journal_line_table = get_validated_table_name(schema, "JournalLine", "journal_line")?;
    let journal_entry_table = get_validated_table_name(schema, "JournalEntry", "journal_entry")?;
    // account_table is already validated above

    let sql = format!(
        r#"
        SELECT
            jl.account as account_id,
            jl.debit,
            jl.credit,
            a.type as account_type
        FROM {} jl
        JOIN {} je ON jl.journal_entry = je.id
        JOIN {} a ON jl.account = a.id
        WHERE je.status = 'Posted'
          AND je.date >= {}
          AND je.date <= {}
          AND (a.type = 'Revenue' OR a.type = 'Expense')
    "#,
        journal_line_table, journal_entry_table, account_table, p_start, p_end
    );

    let params_vec = vec![
        Value::String(start_date.to_string()),
        Value::String(end_date.to_string()),
    ];

    let query_result = data_access.datastore().query_with_params(&sql, params_vec).await;

    // Aggregate in memory using Decimal
    let mut account_balances: HashMap<String, (Decimal, Decimal)> = HashMap::new(); // AccountID -> (TotalDebit, TotalCredit)

    let rows_to_process = match query_result {
        Ok(rows) => rows,
        Err(e) if e.contains("not supported") => {
            // Fallback for MemoryDataStore / No-SQL
            let mut fallback_rows = Vec::new();

            // 1. Fetch All Posted JournalEntries (filter by status)
            let mut je_filters = HashMap::new();
            je_filters.insert("status".to_string(), "Posted".to_string());

            let journals = data_access
                .datastore()
                .find(journal_entry_table, je_filters)
                .await
                .map_err(RuntimeError::WorkflowError)?;

            // 2. Filter by Date Range in Memory
            let p_start_date = NaiveDate::parse_from_str(start_date, "%Y-%m-%d")
                .map_err(|_| RuntimeError::ValidationError(format!("Invalid start_date format: '{}'", start_date)))?;
            let p_end_date = NaiveDate::parse_from_str(end_date, "%Y-%m-%d")
                .map_err(|_| RuntimeError::ValidationError(format!("Invalid end_date format: '{}'", end_date)))?;

            let mut journal_ids = Vec::new();
            for je in journals {
                let d_str_opt = je.get("date").and_then(|v| v.as_str());
                let id_opt = je.get("id").and_then(|v| v.as_str());
                if let (Some(d_str), Some(id)) = (d_str_opt, id_opt) {
                    let d = NaiveDate::parse_from_str(d_str, "%Y-%m-%d").map_err(|_| {
                        RuntimeError::ValidationError(format!("Invalid journal entry date format: '{}'", d_str))
                    })?;
                    if d >= p_start_date && d <= p_end_date {
                        journal_ids.push(id.to_string());
                    }
                }
            }

            if !journal_ids.is_empty() {
                // 3. Fetch Accounts to map ID -> Type
                let all_accounts = data_access
                    .datastore()
                    .list(account_table, None, None)
                    .await
                    .map_err(RuntimeError::WorkflowError)?;

                let mut acc_type_map = HashMap::new();
                for acc in all_accounts {
                    if let (Some(id), Some(typ)) = (
                        acc.get("id").and_then(|v| v.as_str()),
                        acc.get("type").and_then(|v| v.as_str()),
                    ) {
                        acc_type_map.insert(id.to_string(), typ.to_string());
                    }
                }

                // 4. Fetch Lines for these journals
                let mut futs = Vec::new();
                for jid in &journal_ids {
                    let mut line_filters = HashMap::new();
                    line_filters.insert("journal_entry".to_string(), jid.clone());
                    futs.push(data_access.datastore().find(journal_line_table, line_filters));
                }

                let results = futures::future::join_all(futs).await;

                for res in results {
                    let lines = res.map_err(RuntimeError::WorkflowError)?;
                    for line in lines {
                        let acc_id_opt = line.get("account").and_then(|v| v.as_str());
                        if let Some(acc_id) = acc_id_opt
                            && let Some(typ) = acc_type_map.get(acc_id)
                        {
                            let is_rev_exp = matches!(typ.as_str(), "Revenue" | "Expense");
                            if is_rev_exp {
                                // Construct a pseudo-row for aggregation
                                let mut row_map = serde_json::Map::new();
                                row_map.insert("account_id".to_string(), Value::String(acc_id.to_string()));
                                row_map.insert("account_type".to_string(), Value::String(typ.to_string()));
                                let debit_val = line.get("debit").cloned().ok_or_else(|| {
                                    RuntimeError::ValidationError(format!(
                                        "Journal line missing debit for account {}",
                                        acc_id
                                    ))
                                })?;
                                let credit_val = line.get("credit").cloned().ok_or_else(|| {
                                    RuntimeError::ValidationError(format!(
                                        "Journal line missing credit for account {}",
                                        acc_id
                                    ))
                                })?;
                                row_map.insert("debit".to_string(), debit_val);
                                row_map.insert("credit".to_string(), credit_val);
                                fallback_rows.push(Arc::new(Value::Object(row_map)));
                            }
                        }
                    }
                }
            }

            fallback_rows
        }
        Err(e) => return Err(RuntimeError::WorkflowError(e)),
    };

    for row in rows_to_process {
        let account_id = row
            .get("account_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| RuntimeError::ValidationError("Row missing account_id".to_string()))?
            .to_string();
        if account_id.is_empty() {
            continue;
        }

        let debit = parse_decimal_opt(row.get("debit"))?;
        let credit = parse_decimal_opt(row.get("credit"))?;

        let entry = account_balances
            .entry(account_id)
            .or_insert((Decimal::ZERO, Decimal::ZERO));
        entry.0 += debit;
        entry.1 += credit;
    }

    let mut closing_lines = vec![];
    let mut total_retained_earnings_impact = Decimal::ZERO; // Positive = Credit to RE (Profit)

    for (account_id, (total_debit, total_credit)) in account_balances {
        let net = total_debit - total_credit;

        if net.is_zero() {
            continue;
        }

        let mut line = serde_json::Map::new();
        line.insert("account".to_string(), Value::String(account_id));

        if net > Decimal::ZERO {
            // Debit Balance (Expense) -> Credit it
            line.insert("credit".to_string(), Value::String(net.to_string()));
            line.insert("debit".to_string(), Value::String("0.00".to_string()));

            // Expense reduces Equity. We Credit Expense, so we Debit RE.
            // Impact tracks Credit side. So Debit = Negative Impact.
            total_retained_earnings_impact -= net;
        } else {
            // Credit Balance (Revenue) -> Debit it
            let amount = net.abs();
            line.insert("debit".to_string(), Value::String(amount.to_string()));
            line.insert("credit".to_string(), Value::String("0.00".to_string()));

            // Revenue increases Equity. We Debit Revenue, so we Credit RE.
            // Impact tracks Credit side.
            total_retained_earnings_impact += amount;
        }
        closing_lines.push(Value::Object(line));
    }

    if closing_lines.is_empty() {
        return Ok(true);
    }

    // Add Plug Line for Retained Earnings
    let impact = total_retained_earnings_impact;

    if !impact.is_zero() {
        let mut plug_line = serde_json::Map::new();
        plug_line.insert("account".to_string(), Value::String(retained_earnings_id.to_string()));

        if impact > Decimal::ZERO {
            // Profit -> Credit RE
            plug_line.insert("credit".to_string(), Value::String(impact.to_string()));
            plug_line.insert("debit".to_string(), Value::String("0.00".to_string()));
        } else {
            // Loss -> Debit RE
            plug_line.insert("debit".to_string(), Value::String(impact.abs().to_string()));
            plug_line.insert("credit".to_string(), Value::String("0.00".to_string()));
        }
        closing_lines.push(Value::Object(plug_line));
    }

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
    for line_val in &mut closing_lines {
        if let Some(obj) = line_val.as_object_mut() {
            obj.insert("journal_entry".to_string(), Value::String(new_journal_id.clone()));
        }
    }

    data_access
        .create_many("JournalLine", closing_lines, ctx)
        .await
        .map_err(RuntimeError::WorkflowError)?;

    Ok(true)
}

async fn execute_snapshot_parties(
    entity_data: &Value,
    schema: &Schema,
    datastore: Option<&Arc<dyn DataStore>>,
) -> Result<(), RuntimeError> {
    if let (Some(ds), Some(journal_id)) = (datastore, entity_data.get("id").and_then(|v| v.as_str())) {
        // 1. Fetch Journal Lines
        let table_name = get_validated_table_name(schema, "JournalLine", "journal_line")?;

        let mut filters = HashMap::new();
        filters.insert("journal_entry".to_string(), journal_id.to_string());

        let lines = ds
            .find(table_name, filters)
            .await
            .map_err(RuntimeError::WorkflowError)?;

        // 2. Identify journal lines that require party name snapshots
        let mut parties_to_fetch: HashMap<String, std::collections::HashSet<String>> = HashMap::new();
        for line_arc in &lines {
            let line = line_arc.as_ref();
            let party_type = line.get("party_type").and_then(|v| v.as_str());
            let party_id = line.get("party_id").and_then(|v| v.as_str());
            let current_name = line.get("party_name").and_then(|v| v.as_str());

            let is_empty = current_name.is_none_or(|n| n.is_empty());
            if let (Some(pt), Some(pid), true) = (party_type, party_id, is_empty) {
                parties_to_fetch
                    .entry(pt.to_string())
                    .or_default()
                    .insert(pid.to_string());
            }
        }

        if parties_to_fetch.is_empty() {
            return Ok(());
        }

        // 3. Batch fetch party names for each identified party type
        let db_type = get_db_type(schema);

        let mut fetch_futs = Vec::new();

        for (pt, pids) in parties_to_fetch {
            if let Ok(target_table) = get_validated_table_name_strict(schema, &pt) {
                let ids: Vec<String> = pids.into_iter().collect();
                let placeholders = (1..=ids.len())
                    .map(|i| gurih_ir::utils::get_db_placeholder(&db_type, i))
                    .collect::<Vec<_>>()
                    .join(", ");

                let sql = format!(
                    "SELECT id, name, full_name, description FROM {} WHERE id IN ({})",
                    target_table, placeholders
                );
                let params: Vec<Value> = ids.iter().map(|s| Value::String(s.clone())).collect();

                let ds = ds.clone();
                let pt = pt.clone();

                fetch_futs.push(async move {
                    let mut local_cache = Vec::new();
                    match ds.query_with_params(&sql, params).await {
                        Ok(results) => {
                            for record in results {
                                if let Some(id) = record.get("id").and_then(|v| v.as_str()) {
                                    let name = record
                                        .get("name")
                                        .or_else(|| record.get("full_name"))
                                        .or_else(|| record.get("description"))
                                        .and_then(|v| v.as_str())
                                        .ok_or_else(|| {
                                            RuntimeError::ValidationError(format!("Party {} missing name", id))
                                        })?;
                                    local_cache.push(((pt.clone(), id.to_string()), name.to_string()));
                                }
                            }
                            Ok(local_cache)
                        }
                        Err(e) if e.contains("Raw SQL query not supported") => {
                            // Fallback: Parallel fetch
                            let mut futs = Vec::new();
                            for id in &ids {
                                futs.push(ds.get(target_table, id));
                            }
                            let results = join_all(futs).await;
                            for (id, res) in ids.iter().zip(results) {
                                if let Ok(Some(party_record)) = res {
                                    let name = party_record
                                        .get("name")
                                        .or_else(|| party_record.get("full_name"))
                                        .or_else(|| party_record.get("description"))
                                        .and_then(|v| v.as_str())
                                        .ok_or_else(|| {
                                            RuntimeError::ValidationError(format!("Party {} missing name", id))
                                        })?;
                                    local_cache.push(((pt.clone(), id.to_string()), name.to_string()));
                                }
                            }
                            Ok(local_cache)
                        }
                        Err(e) => Err(RuntimeError::WorkflowError(e)),
                    }
                });
            }
        }

        let mut party_names_cache: HashMap<(String, String), String> = HashMap::new();
        let fetch_results = join_all(fetch_futs).await;
        for res in fetch_results {
            let local_cache = res?;
            for (key, val) in local_cache {
                party_names_cache.insert(key, val);
            }
        }

        // 4. Execute journal line updates in parallel
        let mut update_futs = Vec::new();
        for line_arc in &lines {
            let line = line_arc.as_ref();
            let line_id = line.get("id").and_then(|v| v.as_str());
            let party_type = line.get("party_type").and_then(|v| v.as_str());
            let party_id = line.get("party_id").and_then(|v| v.as_str());
            let current_name = line.get("party_name").and_then(|v| v.as_str());

            let is_empty = current_name.is_none_or(|n| n.is_empty());
            if let (Some(lid), Some(pt), Some(pid), true) = (line_id, party_type, party_id, is_empty) {
                #[allow(clippy::collapsible_if)]
                if let Some(name) = party_names_cache.get(&(pt.to_string(), pid.to_string())) {
                    let mut update_data = serde_json::Map::new();
                    update_data.insert("party_name".to_string(), Value::String(name.clone()));
                    update_futs.push(ds.update(table_name, lid, Value::Object(update_data)));
                }
            }
        }

        if !update_futs.is_empty() {
            let update_results = futures::future::join_all(update_futs).await;
            for res in update_results {
                res.map_err(RuntimeError::WorkflowError)?;
            }
        }
    }
    Ok(())
}
