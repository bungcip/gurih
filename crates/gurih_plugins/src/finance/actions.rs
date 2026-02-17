use gurih_ir::utils::{get_db_range_placeholders, parse_numeric_opt, resolve_param};
use gurih_ir::{ActionStep, Symbol};
use gurih_runtime::context::RuntimeContext;
use gurih_runtime::errors::RuntimeError;
use gurih_runtime::traits::DataAccess;
use serde_json::{Value, json};
use std::collections::HashMap;
use uuid::Uuid;

pub async fn execute_reverse_journal(
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

            let debit = parse_numeric_opt(obj.get("debit"));
            let credit = parse_numeric_opt(obj.get("credit"));

            obj.insert("debit".to_string(), json!(credit.to_string()));
            obj.insert("credit".to_string(), json!(debit.to_string()));
        }
        reverse_lines.push(line);
    }

    data_access
        .create_many("JournalLine", reverse_lines, ctx)
        .await
        .map_err(RuntimeError::WorkflowError)?;

    Ok(true)
}

pub async fn execute_generate_closing_entry(
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
            "Retained Earnings account not found. Please ensure an account with system_tag='retained_earnings' exists.".to_string(),
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
