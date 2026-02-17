use chrono::NaiveDate;
use gurih_ir::utils::{get_db_placeholder, get_db_range_placeholders, parse_numeric_opt};
use gurih_ir::{Expression, Schema, Symbol};
use gurih_runtime::datastore::DataStore;
use gurih_runtime::errors::RuntimeError;
use gurih_runtime::store::validate_identifier;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;

use super::utils::fetch_journal_lines;

pub async fn check_balanced_transaction(
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

pub async fn check_valid_parties(
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
            .map(|i| get_db_placeholder(&db_type, i))
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
                .map(|i| get_db_placeholder(&db_type, i))
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

pub async fn check_period_open(
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
