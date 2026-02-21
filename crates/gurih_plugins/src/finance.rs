use async_trait::async_trait;
use chrono::NaiveDate;
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

fn parse_decimal_opt(v: Option<&Value>) -> Decimal {
    match v {
        Some(val) => {
            if let Some(s) = val.as_str() {
                s.parse().unwrap_or(Decimal::ZERO)
            } else if let Some(n) = val.as_f64() {
                // Best effort conversion from float
                Decimal::from_f64_retain(n).unwrap_or(Decimal::ZERO)
            } else {
                Decimal::ZERO
            }
        },
        None => Decimal::ZERO,
    }
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
        if name == "post_journal"
            && let Some(Expression::StringLiteral(rule)) = args.first()
        {
            return Ok((Value::Null, vec![], vec![Symbol::from(rule.as_str())]));
        } else if name == "snapshot_parties" {
            execute_snapshot_parties(entity_data, schema, datastore).await?;
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

    let mut total_debit = Decimal::ZERO;
    let mut total_credit = Decimal::ZERO;

    for line in lines {
        if let Some(line_obj) = line.as_object() {
            total_debit += parse_decimal_opt(line_obj.get("debit"));
            total_credit += parse_decimal_opt(line_obj.get("credit"));
        }
    }

    let diff = (total_debit - total_credit).abs();
    if !diff.is_zero() {
        return Err(RuntimeError::ValidationError(format!(
            "Transaction not balanced: Debit {}, Credit {} (Diff {})",
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

    let db_type = schema
        .database
        .as_ref()
        .map(|d| d.db_type.clone())
        .unwrap_or(gurih_ir::DatabaseType::Sqlite);

    // Cache: Account ID -> Account Data
    let mut accounts_cache: HashMap<String, Arc<Value>> = HashMap::new();
    // Cache: (PartyType, PartyID) -> Exists (bool)
    let mut party_existence_cache: HashMap<(String, String), bool> = HashMap::new();

    // 1. Batch Fetch Accounts
    if !account_ids.is_empty() {
        let account_table = schema
            .entities
            .get(&Symbol::from("Account"))
            .map(|e| e.table_name.as_str())
            .unwrap_or("account");

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
                // Fallback: Fetch one by one (MemoryDataStore)
                for id in ids {
                    if let Some(acc) = ds.get(account_table, &id).await.map_err(RuntimeError::WorkflowError)? {
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
        if let Some(entity_schema) = target_entity {
            let table = entity_schema.table_name.as_str();
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
                    // Fallback
                    for id in ids {
                        let exists = ds.get(table, &id).await.map_err(RuntimeError::WorkflowError)?.is_some();
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
    for line in lines_to_check {
        let account_id = line
            .get("account")
            .or_else(|| line.get("account_id")) // Support both forms
            .and_then(|v| v.as_str())
            .ok_or(RuntimeError::ValidationError(
                "Journal line missing account".to_string(),
            ))?;

        let account = accounts_cache
            .get(account_id)
            .ok_or_else(|| RuntimeError::ValidationError(format!("Account not found: {}", account_id)))?;

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
            let target_entity = schema.entities.get(&Symbol::from(pt));
            if target_entity.is_some() {
                // Check cache
                if !party_existence_cache.contains_key(&(pt.to_string(), pid.to_string())) {
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

async fn check_period_overlap(
    entity_data: &Value,
    schema: &Schema,
    datastore: Option<&Arc<dyn DataStore>>,
) -> Result<(), RuntimeError> {
    let ds = datastore
        .ok_or_else(|| RuntimeError::WorkflowError("Datastore not available".to_string()))?;

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
        .map_err(|_| RuntimeError::ValidationError("Invalid start_date format".to_string()))?;
    let end_date = NaiveDate::parse_from_str(end_date_s, "%Y-%m-%d")
        .map_err(|_| RuntimeError::ValidationError("Invalid end_date format".to_string()))?;

    if start_date > end_date {
        return Err(RuntimeError::ValidationError(
            "Start date must be before or equal to end date".to_string(),
        ));
    }

    let current_id = entity_data.get("id").and_then(|v| v.as_str());

    // 2. Fetch all periods
    let table_name = schema
        .entities
        .get(&Symbol::from("AccountingPeriod"))
        .map(|e| e.table_name.as_str())
        .unwrap_or("accounting_period");

    let all_periods = ds
        .list(table_name, None, None)
        .await
        .map_err(RuntimeError::WorkflowError)?;

    // 3. Check Overlap
    for period in all_periods {
        // Skip self
        if let Some(pid) = period.get("id").and_then(|v| v.as_str()) {
            if let Some(cid) = current_id {
                if pid == cid {
                    continue;
                }
            }
        }

        // Skip Draft
        if let Some(status) = period.get("status").and_then(|v| v.as_str()) {
            if status == "Draft" {
                continue;
            }
        }

        let p_start_s = period
            .get("start_date")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let p_end_s = period
            .get("end_date")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        if let (Ok(p_start), Ok(p_end)) = (
            NaiveDate::parse_from_str(p_start_s, "%Y-%m-%d"),
            NaiveDate::parse_from_str(p_end_s, "%Y-%m-%d"),
        ) {
            // Overlap logic: start1 <= end2 AND end1 >= start2
            if start_date <= p_end && end_date >= p_start {
                let name = period.get("name").and_then(|v| v.as_str()).unwrap_or("?");
                return Err(RuntimeError::ValidationError(format!(
                    "Period overlaps with existing period '{}'",
                    name
                )));
            }
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
    let id_raw = step.args.get("id").ok_or(RuntimeError::WorkflowError(
        "Missing 'id' argument for finance:reverse_journal".to_string(),
    ))?;
    let id = resolve_param(id_raw, params);

    // 1. Read Original
    let original_arc = data_access
        .read("JournalEntry", &id, ctx)
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
    let mut reverse_lines = Vec::with_capacity(lines.len());
    for line_arc in lines {
        let mut line = line_arc.as_ref().clone();
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
        .ok_or(RuntimeError::WorkflowError("Missing 'period_id' argument".to_string()))?;
    let period_id = resolve_param(period_id_raw, params);

    // 1. Fetch Period
    let period_arc = data_access
        .read("AccountingPeriod", &period_id, ctx)
        .await
        .map_err(RuntimeError::WorkflowError)?
        .ok_or(RuntimeError::WorkflowError("AccountingPeriod not found".to_string()))?;

    let start_date = period_arc.get("start_date").and_then(|v| v.as_str()).unwrap_or("");
    let end_date = period_arc.get("end_date").and_then(|v| v.as_str()).unwrap_or("");
    let period_name = period_arc.get("name").and_then(|v| v.as_str()).unwrap_or("");

    // 2. Find Retained Earnings Account
    let mut filters = HashMap::new();
    filters.insert("system_tag".to_string(), "retained_earnings".to_string());

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
            "Retained Earnings account not found. Please ensure an account with system_tag='retained_earnings' exists."
                .to_string(),
        ))?;

    // 3. Aggregate Revenue and Expense
    // We fetch raw lines instead of SUM() to ensure decimal precision
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

    let results = data_access
        .datastore()
        .query_with_params(&sql, params_vec)
        .await
        .map_err(RuntimeError::WorkflowError)?;

    // Aggregate in memory using Decimal
    let mut account_balances: HashMap<String, (Decimal, Decimal)> = HashMap::new(); // AccountID -> (TotalDebit, TotalCredit)
    let mut account_types: HashMap<String, String> = HashMap::new();

    for row in results {
        let account_id = row.get("account_id").and_then(|v| v.as_str()).unwrap_or("").to_string();
        if account_id.is_empty() { continue; }

        let debit = parse_decimal_opt(row.get("debit"));
        let credit = parse_decimal_opt(row.get("credit"));

        if let Some(t) = row.get("account_type").and_then(|v| v.as_str()) {
            account_types.insert(account_id.clone(), t.to_string());
        }

        let entry = account_balances.entry(account_id).or_insert((Decimal::ZERO, Decimal::ZERO));
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
    let mut plug_line = serde_json::Map::new();
    plug_line.insert("account".to_string(), Value::String(retained_earnings_id.to_string()));

    let impact = total_retained_earnings_impact;

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
    if let Some(ds) = datastore
        && let Some(journal_id) = entity_data.get("id").and_then(|v| v.as_str())
    {
        // 1. Fetch Journal Lines
        let table_name = schema
            .entities
            .get(&Symbol::from("JournalLine"))
            .map(|e| e.table_name.as_str())
            .unwrap_or("journal_line");

        let mut filters = HashMap::new();
        filters.insert("journal_entry".to_string(), journal_id.to_string());

        let lines = ds
            .find(table_name, filters)
            .await
            .map_err(RuntimeError::WorkflowError)?;

        // 2. Iterate and Update
        for line_arc in lines {
            let line = line_arc.as_ref();
            let line_id = line.get("id").and_then(|v| v.as_str());

            if let Some(lid) = line_id {
                let party_type = line.get("party_type").and_then(|v| v.as_str());
                let party_id = line.get("party_id").and_then(|v| v.as_str());
                let current_name = line.get("party_name").and_then(|v| v.as_str());

                // Only update if party_id exists AND party_name is missing/empty
                if let (Some(pt), Some(pid)) = (party_type, party_id) {
                    if current_name.is_none() || current_name.unwrap().is_empty() {
                        // Fetch Party Name
                        if let Some(target_entity) = schema.entities.get(&Symbol::from(pt)) {
                            let target_table = target_entity.table_name.as_str();
                            if let Some(party_record) =
                                ds.get(target_table, pid).await.map_err(RuntimeError::WorkflowError)?
                            {
                                let name = party_record
                                    .get("name")
                                    .or_else(|| party_record.get("full_name")) // Try common name fields
                                    .or_else(|| party_record.get("description"))
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("Unknown");

                                // Update JournalLine
                                let mut update_data = serde_json::Map::new();
                                update_data.insert("party_name".to_string(), Value::String(name.to_string()));

                                ds.update(table_name, lid, Value::Object(update_data))
                                    .await
                                    .map_err(RuntimeError::WorkflowError)?;
                            }
                        }
                    }
                }
            }
        }
    }
    Ok(())
}
