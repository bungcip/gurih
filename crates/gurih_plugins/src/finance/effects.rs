use gurih_ir::{Schema, Symbol};
use gurih_runtime::datastore::DataStore;
use gurih_runtime::errors::RuntimeError;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;

pub async fn execute_snapshot_parties(
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
                            if let Some(party_record) = ds
                                .get(target_table, pid)
                                .await
                                .map_err(RuntimeError::WorkflowError)?
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
