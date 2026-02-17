use gurih_ir::{Schema, Symbol};
use gurih_runtime::datastore::DataStore;
use gurih_runtime::errors::RuntimeError;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;

pub async fn fetch_journal_lines(
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
