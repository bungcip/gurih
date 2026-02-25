use crate::auth::hash_password;
use crate::context::RuntimeContext;
use crate::datastore::DataStore;
use crate::plugins::Plugin;
use crate::query_engine::{QueryEngine, QueryPlan};
use crate::traits::DataAccess;
use crate::workflow::WorkflowEngine;
use async_trait::async_trait;
use chrono::Local;
use futures::future::join_all;
use gurih_ir::{FieldType, QueryJoin, Schema, Symbol};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;

pub struct DataEngine {
    schema: Arc<Schema>,
    datastore: Arc<dyn DataStore>,
    workflow: WorkflowEngine,
}

struct HierarchyContext<'a> {
    records_map: &'a std::collections::HashMap<String, Arc<Value>>,
    children_map: &'a std::collections::HashMap<String, Vec<String>>,
    rollups_cache: &'a std::collections::HashMap<String, serde_json::Map<String, Value>>,
}

#[async_trait]
impl DataAccess for DataEngine {
    fn get_schema(&self) -> &Schema {
        &self.schema
    }

    fn datastore(&self) -> &Arc<dyn DataStore> {
        &self.datastore
    }

    async fn create(&self, entity_name: &str, data: Value, ctx: &RuntimeContext) -> Result<String, String> {
        self.create(entity_name, data, ctx).await
    }

    async fn create_many(
        &self,
        entity_name: &str,
        data: Vec<Value>,
        ctx: &RuntimeContext,
    ) -> Result<Vec<String>, String> {
        self.create_many(entity_name, data, ctx).await
    }

    async fn read(&self, entity_name: &str, id: &str, ctx: &RuntimeContext) -> Result<Option<Arc<Value>>, String> {
        self.read(entity_name, id, ctx).await
    }

    async fn update(&self, entity_name: &str, id: &str, data: Value, ctx: &RuntimeContext) -> Result<(), String> {
        self.update(entity_name, id, data, ctx).await
    }

    async fn delete(&self, entity_name: &str, id: &str, ctx: &RuntimeContext) -> Result<(), String> {
        self.delete(entity_name, id, ctx).await
    }

    async fn list(
        &self,
        entity: &str,
        limit: Option<usize>,
        offset: Option<usize>,
        filters: Option<HashMap<String, String>>,
        ctx: &RuntimeContext,
    ) -> Result<Vec<Arc<Value>>, String> {
        self.list(entity, limit, offset, filters, ctx).await
    }
}

impl DataEngine {
    pub fn new(schema: Arc<Schema>, datastore: Arc<dyn DataStore>) -> Self {
        Self {
            schema,
            datastore,
            workflow: WorkflowEngine::new(),
        }
    }

    pub fn with_plugins(mut self, plugins: Vec<Box<dyn Plugin>>) -> Self {
        self.workflow = self.workflow.with_plugins(plugins);
        self
    }

    pub fn get_schema(&self) -> &Schema {
        &self.schema
    }

    pub fn datastore(&self) -> &Arc<dyn DataStore> {
        &self.datastore
    }

    async fn check_rules(
        &self,
        entity_name: &str,
        action: &str,
        new_data: &Value,
        old_data: Option<&Value>,
    ) -> Result<(), String> {
        let event = format!("{}:{}", entity_name, action);
        let event_sym = Symbol::from(&event);

        // Construct context with self and old
        let mut context_map = if let Some(obj) = new_data.as_object() {
            obj.clone()
        } else {
            serde_json::Map::new()
        };

        context_map.insert("self".to_string(), new_data.clone());
        if let Some(old) = old_data {
            context_map.insert("old".to_string(), old.clone());
        } else {
            context_map.insert("old".to_string(), Value::Null);
        }
        let context = Value::Object(context_map);

        for rule in self.schema.rules.values() {
            if rule.on_event == event_sym {
                let result =
                    crate::evaluator::evaluate(&rule.assertion, &context, Some(&self.schema), Some(&self.datastore))
                        .await
                        .map_err(|e| format!("Rule '{}' error: {}", rule.name, e))?;

                match result {
                    Value::Bool(true) => continue,
                    Value::Bool(false) => return Err(rule.message.clone()),
                    _ => return Err(format!("Rule '{}' assertion must return a boolean", rule.name)),
                }
            }
        }
        Ok(())
    }

    async fn generate_serial_number(&self, generator_name: &Symbol, _ctx: &RuntimeContext) -> Result<String, String> {
        let generator = self
            .schema
            .serial_generators
            .get(generator_name)
            .ok_or_else(|| format!("Serial generator '{}' not found", generator_name))?;

        let now = Local::now();
        let mut prefix = generator.prefix.clone().unwrap_or_default();

        if let Some(fmt) = &generator.date_format {
            // Apply date format
            let date_part = if fmt.contains('%') {
                now.format(fmt).to_string()
            } else {
                let yyyy = now.format("%Y").to_string();
                let mm = now.format("%m").to_string();
                let dd = now.format("%d").to_string();
                fmt.replace("YYYY", &yyyy).replace("MM", &mm).replace("DD", &dd)
            };
            prefix.push_str(&date_part);
        }

        // Context key for sequence
        let context_key = prefix.clone();
        let seq_name = generator.name.as_str();

        // Atomic Increment
        let new_val = self.next_sequence_value(seq_name, &context_key).await?;

        // Format
        let seq_str = format!("{:0width$}", new_val, width = generator.digits as usize);
        if prefix.is_empty() {
            Ok(seq_str)
        } else {
            Ok(format!("{}{}", prefix, seq_str))
        }
    }

    async fn next_sequence_value(&self, name: &str, context: &str) -> Result<i64, String> {
        let db_type = self.schema.database.as_ref().map(|d| d.db_type.clone());

        if let Some(db_t) = db_type {
            let sql = if db_t == gurih_ir::DatabaseType::Postgres {
                r#"INSERT INTO "_gurih_sequences" ("name", "context", "value") VALUES ($1, $2, 1) ON CONFLICT ("name", "context") DO UPDATE SET "value" = "_gurih_sequences"."value" + 1 RETURNING "value""#
            } else {
                r#"INSERT INTO _gurih_sequences (name, context, value) VALUES ($1, $2, 1) ON CONFLICT(name, context) DO UPDATE SET value = value + 1 RETURNING value"#
            };

            let params = vec![Value::String(name.to_string()), Value::String(context.to_string())];

            match self.datastore.query_with_params(sql, params).await {
                Ok(rows) => {
                    if let Some(row) = rows.first() {
                        if let Some(val) = row.get("value").and_then(|v| v.as_i64()) {
                            return Ok(val);
                        } else if let Some(val) = row.get("value").and_then(|v| v.as_str()) {
                            // SQLite sometimes returns numbers as strings if mapped incorrectly or dynamic
                            return val.parse::<i64>().map_err(|e| e.to_string());
                        }
                    }
                    Err("Failed to return sequence value".to_string())
                }
                Err(e) => {
                    if e.contains("not supported") {
                        // Fallback for MemoryDataStore
                        self.next_sequence_fallback(name, context).await
                    } else {
                        Err(e)
                    }
                }
            }
        } else {
            // No DB configured, assume Memory
            self.next_sequence_fallback(name, context).await
        }
    }

    async fn next_sequence_fallback(&self, name: &str, context: &str) -> Result<i64, String> {
        let mut filters = HashMap::new();
        filters.insert("name".to_string(), name.to_string());
        filters.insert("context".to_string(), context.to_string());

        if let Some(existing) = self.datastore.find_first("_gurih_sequences", filters.clone()).await? {
            let current = existing.get("value").and_then(|v| v.as_i64()).unwrap_or(0);
            let next = current + 1;
            let id = existing
                .get("id")
                .and_then(|v| v.as_str())
                .ok_or("Sequence record missing ID")?;

            let mut update_data = serde_json::Map::new();
            update_data.insert("value".to_string(), Value::from(next));

            self.datastore
                .update("_gurih_sequences", id, Value::Object(update_data))
                .await?;
            Ok(next)
        } else {
            let mut new_record = serde_json::Map::new();
            new_record.insert("name".to_string(), Value::String(name.to_string()));
            new_record.insert("context".to_string(), Value::String(context.to_string()));
            new_record.insert("value".to_string(), Value::from(1));

            self.datastore
                .insert("_gurih_sequences", Value::Object(new_record))
                .await?;
            Ok(1)
        }
    }

    pub async fn create(&self, entity_name: &str, mut data: Value, ctx: &RuntimeContext) -> Result<String, String> {
        let entity_schema = self
            .schema
            .entities
            .get(&Symbol::from(entity_name))
            .ok_or_else(|| format!("Entity '{}' not defined", entity_name))?;

        // Validate create permission
        let create_perm = entity_schema
            .options
            .get("create_permission")
            .cloned()
            .unwrap_or_else(|| format!("create:{}", entity_name));

        self.validate_permission(ctx, &create_perm, "create", entity_name)?;

        // Rule Check (Create)
        self.check_rules(entity_name, "create", &data, None).await?;

        // Check Composition Immutability (Prevent creating into locked parent)
        self.check_composition_immutability(entity_name, &data, ctx).await?;

        // Workflow: Set initial state if applicable
        if let Some(wf) = self
            .schema
            .workflows
            .values()
            .find(|w| w.entity == Symbol::from(entity_name))
            && let Some(obj) = data.as_object_mut()
            && !obj.contains_key(wf.field.as_str())
        {
            obj.insert(wf.field.to_string(), Value::String(wf.initial_state.to_string()));
        }

        // Validation & Transformation (Hashing)
        if let Some(obj) = data.as_object_mut() {
            // Ensure ID exists (for stores that don't auto-generate, like SQLite with TEXT PK)
            if !obj.contains_key("id") {
                obj.insert("id".to_string(), Value::String(Uuid::new_v4().to_string()));
            }

            // Generate Serials
            for field in &entity_schema.fields {
                if field.field_type == FieldType::Serial {
                    // Generate if not provided (allows manual override if key exists, or generate if missing)
                    // Usually serials are system generated. We assume if it's missing or empty string, we generate.
                    let needs_generation = !obj.contains_key(field.name.as_str())
                        || obj
                            .get(field.name.as_str())
                            .and_then(|v| v.as_str())
                            .map(|s| s.is_empty())
                            .unwrap_or(true);

                    #[allow(clippy::collapsible_if)]
                    if needs_generation {
                        if let Some(gen_name) = &field.serial_generator {
                            let val = self.generate_serial_number(gen_name, ctx).await?;
                            obj.insert(field.name.to_string(), Value::String(val));
                        }
                    }
                }
            }

            for field in &entity_schema.fields {
                if field.required && !obj.contains_key(&field.name.to_string()) {
                    return Err(format!("Missing required field: {}", field.name));
                }
            }
            self.process_data_fields(entity_schema, obj)?;
        } else {
            return Err("Data must be an object".to_string());
        }

        let id = self
            .datastore
            .insert(entity_schema.table_name.as_str(), data.clone())
            .await?;

        // Audit Trail
        if let Some(val) = entity_schema.options.get("track_changes")
            && val == "true"
        {
            let diff = serde_json::to_string(&data).unwrap_or_default();
            self.log_audit(entity_name, &id, "CREATE", ctx, Some(diff)).await?;
        }

        Ok(id)
    }

    pub async fn create_many(
        &self,
        entity_name: &str,
        data: Vec<Value>,
        ctx: &RuntimeContext,
    ) -> Result<Vec<String>, String> {
        if data.is_empty() {
            return Ok(vec![]);
        }

        let entity_schema = self
            .schema
            .entities
            .get(&Symbol::from(entity_name))
            .ok_or_else(|| format!("Entity '{}' not defined", entity_name))?;

        // Validate create permission (once for the batch)
        let create_perm = entity_schema
            .options
            .get("create_permission")
            .cloned()
            .unwrap_or_else(|| format!("create:{}", entity_name));

        self.validate_permission(ctx, &create_perm, "create", entity_name)?;

        let mut prepared_records = Vec::with_capacity(data.len());

        // Audit log collection
        let track_changes = entity_schema
            .options
            .get("track_changes")
            .map(|v| v == "true")
            .unwrap_or(false);

        // Phase 1: Parallel Validation
        // Process in chunks to limit concurrency
        for chunk in data.chunks(50) {
            let validation_futures = chunk.iter().map(|record| async move {
                self.check_rules(entity_name, "create", record, None).await?;
                self.check_composition_immutability(entity_name, record, ctx).await
            });

            let validation_results = join_all(validation_futures).await;
            for res in validation_results {
                res?;
            }
        }

        // Phase 2: Sequential Processing (Mutation & Serial Generation)
        for mut record in data {
            // Workflow: Set initial state if applicable
            if let Some(wf) = self
                .schema
                .workflows
                .values()
                .find(|w| w.entity == Symbol::from(entity_name))
                && let Some(obj) = record.as_object_mut()
                && !obj.contains_key(wf.field.as_str())
            {
                obj.insert(wf.field.to_string(), Value::String(wf.initial_state.to_string()));
            }

            if let Some(obj) = record.as_object_mut() {
                // Ensure ID exists
                if !obj.contains_key("id") {
                    obj.insert("id".to_string(), Value::String(Uuid::new_v4().to_string()));
                }

                // Generate Serials
                for field in &entity_schema.fields {
                    if field.field_type == FieldType::Serial {
                        let needs_generation = !obj.contains_key(field.name.as_str())
                            || obj
                                .get(field.name.as_str())
                                .and_then(|v| v.as_str())
                                .map(|s| s.is_empty())
                                .unwrap_or(true);

                        #[allow(clippy::collapsible_if)]
                        if needs_generation {
                            if let Some(gen_name) = &field.serial_generator {
                                let val = self.generate_serial_number(gen_name, ctx).await?;
                                obj.insert(field.name.to_string(), Value::String(val));
                            }
                        }
                    }
                }

                // Check required fields
                for field in &entity_schema.fields {
                    if field.required && !obj.contains_key(&field.name.to_string()) {
                        return Err(format!("Missing required field: {}", field.name));
                    }
                }

                self.process_data_fields(entity_schema, obj)?;
            } else {
                return Err("Data must be an object".to_string());
            }

            prepared_records.push(record);
        }

        let mut all_keys = std::collections::HashSet::new();
        for r in &prepared_records {
            if let Some(obj) = r.as_object() {
                for k in obj.keys() {
                    all_keys.insert(k.clone());
                }
            }
        }

        for r in &mut prepared_records {
            if let Some(obj) = r.as_object_mut() {
                for k in &all_keys {
                    if !obj.contains_key(k) {
                        obj.insert(k.clone(), Value::Null);
                    }
                }
            }
        }

        let ids = self
            .datastore
            .insert_many(entity_schema.table_name.as_str(), prepared_records.clone())
            .await?;

        // Audit Trail
        if track_changes {
            let mut audit_logs = Vec::with_capacity(ids.len());
            for (i, id) in ids.iter().enumerate() {
                let record = &prepared_records[i];
                let diff = serde_json::to_string(record).unwrap_or_default();

                let mut audit_record = serde_json::Map::new();
                audit_record.insert("id".to_string(), Value::String(Uuid::new_v4().to_string()));
                audit_record.insert("entity".to_string(), Value::String(entity_name.to_string()));
                audit_record.insert("record_id".to_string(), Value::String(id.to_string()));
                audit_record.insert("action".to_string(), Value::String("CREATE".to_string()));
                audit_record.insert("user_id".to_string(), Value::String(ctx.user_id.clone()));
                audit_record.insert("diff".to_string(), Value::String(diff));

                audit_logs.push(Value::Object(audit_record));
            }

            self.datastore.insert_many("_audit_log", audit_logs).await.ok();
        }

        Ok(ids)
    }

    pub async fn read(&self, entity_name: &str, id: &str, ctx: &RuntimeContext) -> Result<Option<Arc<Value>>, String> {
        let entity_schema = self.schema.entities.get(&Symbol::from(entity_name));
        if let Some(schema) = entity_schema {
            // Validate read permission
            let read_perm = schema
                .options
                .get("read_permission")
                .cloned()
                .unwrap_or_else(|| format!("read:{}", entity_name));

            self.validate_permission(ctx, &read_perm, "read", entity_name)?;

            self.datastore.get(schema.table_name.as_str(), id).await
        } else {
            Err(format!("Entity '{}' not defined", entity_name))
        }
    }

    async fn check_composition_immutability(
        &self,
        entity_name: &str,
        record: &Value,
        ctx: &RuntimeContext,
    ) -> Result<(), String> {
        let entity_schema = self
            .schema
            .entities
            .get(&Symbol::from(entity_name))
            .ok_or_else(|| format!("Entity '{}' not defined", entity_name))?;

        for rel in &entity_schema.relationships {
            if rel.rel_type == gurih_ir::RelationshipType::BelongsTo
                && rel.ownership == gurih_ir::Ownership::Composition
            {
                // Attempt to resolve FK
                let fk_field = format!("{}_id", rel.name);

                let parent_id = record
                    .get(&fk_field)
                    .or_else(|| record.get(rel.name.as_str()))
                    .and_then(|v| v.as_str());

                if let Some(pid) = parent_id {
                    let parent_entity_name = rel.target_entity.as_str();

                    // Fetch Parent
                    if let Some(parent_arc) = self.read(parent_entity_name, pid, ctx).await? {
                        // Check Parent Workflow
                        let parent_workflow = self.schema.workflows.values().find(|w| w.entity == rel.target_entity);

                        if let Some(pwf) = parent_workflow {
                            let p_state = parent_arc
                                .get(pwf.field.as_str())
                                .and_then(|v| v.as_str())
                                .unwrap_or("");

                            if pwf
                                .states
                                .iter()
                                .any(|s| s.name == Symbol::from(p_state) && s.immutable)
                            {
                                return Err(format!(
                                    "Cannot modify record because parent '{}' is in immutable state '{}'",
                                    parent_entity_name, p_state
                                ));
                            }
                        }
                    }
                }
            }
        }
        Ok(())
    }

    pub async fn update(&self, entity_name: &str, id: &str, data: Value, ctx: &RuntimeContext) -> Result<(), String> {
        let entity_schema = self
            .schema
            .entities
            .get(&Symbol::from(entity_name))
            .ok_or_else(|| format!("Entity '{}' not defined", entity_name))?;

        // Validate update permission
        let update_perm = entity_schema
            .options
            .get("update_permission")
            .cloned()
            .unwrap_or_else(|| format!("update:{}", entity_name));

        self.validate_permission(ctx, &update_perm, "update", entity_name)?;

        let mut data = data;

        // Determine if we need to fetch current record
        let workflow = self
            .schema
            .workflows
            .values()
            .find(|w| w.entity == Symbol::from(entity_name));

        let update_event = format!("{}:update", entity_name);
        let update_event_sym = Symbol::from(&update_event);
        let has_update_rules = self.schema.rules.values().any(|r| r.on_event == update_event_sym);

        let track_changes = entity_schema
            .options
            .get("track_changes")
            .map(|v| v == "true")
            .unwrap_or(false);

        let has_composition = entity_schema.relationships.iter().any(|r| {
            r.rel_type == gurih_ir::RelationshipType::BelongsTo && r.ownership == gurih_ir::Ownership::Composition
        });

        let mut current_record_opt: Option<Arc<Value>> = None;

        if workflow.is_some() || has_update_rules || track_changes || has_composition {
            current_record_opt = self.datastore.get(entity_schema.table_name.as_str(), id).await?;
        }

        // Check Composition Immutability
        if has_composition {
            if let Some(current) = &current_record_opt {
                // 1. Check Source (Old Parent)
                self.check_composition_immutability(entity_name, current, ctx).await?;

                // 2. Check Destination (New Parent)
                // Merge data to get potential new FK
                let mut merged = (**current).clone();
                if let Some(target) = merged.as_object_mut()
                    && let Some(source) = data.as_object()
                {
                    for (k, v) in source {
                        target.insert(k.clone(), v.clone());
                    }
                }
                self.check_composition_immutability(entity_name, &merged, ctx).await?;
            } else {
                return Err("Record not found for composition validation".to_string());
            }
        }

        // Rule Check (Update)
        if has_update_rules {
            if let Some(current) = &current_record_opt {
                let mut merged = (**current).clone();
                if let Some(target) = merged.as_object_mut()
                    && let Some(source) = data.as_object()
                {
                    for (k, v) in source {
                        target.insert(k.clone(), v.clone());
                    }
                }
                self.check_rules(entity_name, "update", &merged, Some(&**current))
                    .await?;
            } else {
                return Err("Record not found for rule validation".to_string());
            }
        }

        if let Some(wf) = workflow {
            let current_record = current_record_opt.as_ref().ok_or("Record not found")?;

            let current_state = current_record
                .get(wf.field.as_str())
                .and_then(|v| v.as_str())
                .unwrap_or("");

            // Check Immutability
            #[allow(clippy::collapsible_if)]
            if let Some(state_schema) = wf.states.iter().find(|s| s.name == Symbol::from(current_state)) {
                if state_schema.immutable {
                    if let Some(obj) = data.as_object() {
                        for key in obj.keys() {
                            if key != wf.field.as_str() && key != "id" {
                                return Err(format!(
                                    "Cannot update field '{}' because record is immutable in state '{}'",
                                    key, current_state
                                ));
                            }
                        }
                    }
                }
            }

            if let Some(new_state) = data.get(wf.field.as_str()).and_then(|v| v.as_str()) {
                // Merge data for validation
                let mut merged_record = (**current_record).clone();
                if let Some(target) = merged_record.as_object_mut()
                    && let Some(source) = data.as_object()
                {
                    for (k, v) in source {
                        target.insert(k.clone(), v.clone());
                    }
                }

                // Validate transition logic
                self.workflow
                    .validate_transition(
                        &self.schema,
                        Some(&self.datastore),
                        entity_name,
                        current_state,
                        new_state,
                        &merged_record,
                    )
                    .await
                    .map_err(|e| e.to_string())?;

                // Validate permissions for transition
                if let Some(perm) =
                    self.workflow
                        .get_transition_permission(&self.schema, entity_name, current_state, new_state)
                    && !ctx.has_permission(&perm)
                {
                    return Err(format!("Missing permission '{}' for transition", perm));
                }

                // Apply Side Effects
                let (updates, notifications, postings) = self
                    .workflow
                    .apply_effects(
                        &self.schema,
                        Some(&self.datastore),
                        entity_name,
                        current_state,
                        new_state,
                        &merged_record,
                    )
                    .await;

                for notification in notifications {
                    println!("NOTIFICATION: {}", notification);
                }

                // Execute Posting Rules
                for rule_name in postings {
                    if let Err(e) = self.execute_posting_rule(&rule_name, &merged_record, ctx).await {
                        println!("POSTING ERROR: {}", e);
                        // Optional: fail the transaction?
                        // For now, log error and continue or return error?
                        // If critical, we should probably fail.
                        return Err(format!("Posting failed: {}", e));
                    }
                }

                // Merge updates into data
                if let Some(obj) = data.as_object_mut()
                    && let Value::Object(update_map) = updates
                {
                    for (k, v) in update_map {
                        obj.insert(k, v);
                    }
                }
            }
        }

        // Validation & Transformation (Hashing)
        if let Some(obj) = data.as_object_mut() {
            self.process_data_fields(entity_schema, obj)?;
        }

        self.datastore
            .update(entity_schema.table_name.as_str(), id, data.clone())
            .await?;

        // Audit Trail (Post-update)
        if track_changes && let Some(current) = &current_record_opt {
            let mut changes = serde_json::Map::new();
            if let Some(new_obj) = data.as_object()
                && let Some(old_obj) = current.as_object()
            {
                for (k, new_v) in new_obj {
                    if k == "id" {
                        continue;
                    }
                    let old_v = old_obj.get(k).unwrap_or(&Value::Null);
                    if new_v != old_v {
                        changes.insert(
                            k.clone(),
                            serde_json::json!({
                                "old": old_v,
                                "new": new_v
                            }),
                        );
                    }
                }
            }

            if !changes.is_empty() {
                let diff = serde_json::to_string(&changes).unwrap_or_default();
                self.log_audit(entity_name, id, "UPDATE", ctx, Some(diff)).await?;
            }
        }

        Ok(())
    }

    pub async fn delete(&self, entity_name: &str, id: &str, ctx: &RuntimeContext) -> Result<(), String> {
        let entity_schema = self
            .schema
            .entities
            .get(&Symbol::from(entity_name))
            .ok_or_else(|| format!("Entity '{}' not defined", entity_name))?;

        // Validate delete permission
        let delete_perm = entity_schema
            .options
            .get("delete_permission")
            .cloned()
            .unwrap_or_else(|| format!("delete:{}", entity_name));

        self.validate_permission(ctx, &delete_perm, "delete", entity_name)?;

        // Validate read permission
        let read_perm = entity_schema
            .options
            .get("read_permission")
            .cloned()
            .unwrap_or_else(|| format!("read:{}", entity_name));

        self.validate_permission(ctx, &read_perm, "read", entity_name)?;

        // Check Workflow Immutability
        let workflow = self
            .schema
            .workflows
            .values()
            .find(|w| w.entity == Symbol::from(entity_name));

        let delete_event = format!("{}:delete", entity_name);
        let delete_event_sym = Symbol::from(&delete_event);
        let has_delete_rules = self.schema.rules.values().any(|r| r.on_event == delete_event_sym);

        let track_changes = entity_schema
            .options
            .get("track_changes")
            .map(|v| v == "true")
            .unwrap_or(false);

        let has_composition = entity_schema.relationships.iter().any(|r| {
            r.rel_type == gurih_ir::RelationshipType::BelongsTo && r.ownership == gurih_ir::Ownership::Composition
        });

        let mut current_record_opt: Option<Arc<Value>> = None;

        if workflow.is_some() || has_delete_rules || has_composition {
            current_record_opt = self.datastore.get(entity_schema.table_name.as_str(), id).await?;
        }

        // Check Composition Immutability
        if has_composition {
            if let Some(current) = &current_record_opt {
                self.check_composition_immutability(entity_name, current, ctx).await?;
            } else {
                return Err("Record not found for composition validation".to_string());
            }
        }

        if let Some(wf) = workflow {
            let record = current_record_opt.as_ref().ok_or("Record not found")?;
            let current_state = record.get(wf.field.as_str()).and_then(|v| v.as_str()).unwrap_or("");

            if let Some(state_schema) = wf.states.iter().find(|s| s.name == Symbol::from(current_state))
                && state_schema.immutable
            {
                return Err(format!("Cannot delete record in immutable state '{}'", current_state));
            }
        }

        // Rule Check (Delete)
        if has_delete_rules {
            if let Some(current) = &current_record_opt {
                self.check_rules(entity_name, "delete", current, None).await?;
            } else {
                return Err("Record not found for rule validation".to_string());
            }
        }

        self.datastore.delete(entity_schema.table_name.as_str(), id).await?;

        // Audit Trail
        if track_changes {
            self.log_audit(entity_name, id, "DELETE", ctx, None).await?;
        }

        Ok(())
    }

    fn validate_permission(
        &self,
        ctx: &RuntimeContext,
        permission: &str,
        action: &str,
        entity_name: &str,
    ) -> Result<(), String> {
        if !ctx.has_permission(permission) {
            Err(format!(
                "Missing permission '{}' to {} entity '{}'",
                permission, action, entity_name
            ))
        } else {
            Ok(())
        }
    }

    async fn log_audit(
        &self,
        entity_name: &str,
        id: &str,
        action: &str,
        ctx: &RuntimeContext,
        diff: Option<String>,
    ) -> Result<(), String> {
        let audit_id = Uuid::new_v4().to_string();
        let mut audit_record = serde_json::Map::new();
        audit_record.insert("id".to_string(), Value::String(audit_id));
        audit_record.insert("entity".to_string(), Value::String(entity_name.to_string()));
        audit_record.insert("record_id".to_string(), Value::String(id.to_string()));
        audit_record.insert("action".to_string(), Value::String(action.to_string()));
        audit_record.insert("user_id".to_string(), Value::String(ctx.user_id.clone()));

        if let Some(d) = diff {
            audit_record.insert("diff".to_string(), Value::String(d));
        } else {
            audit_record.insert("diff".to_string(), Value::Null);
        }

        self.datastore
            .insert("_audit_log", Value::Object(audit_record))
            .await
            .map(|_| ())
    }

    fn process_data_fields(
        &self,
        entity_schema: &gurih_ir::EntitySchema,
        obj: &mut serde_json::Map<String, Value>,
    ) -> Result<(), String> {
        for field in &entity_schema.fields {
            if let Some(val) = obj.get_mut(field.name.as_str()) {
                if !crate::validation::validate_type(val, &field.field_type) {
                    return Err(format!("Invalid type for field: {}", field.name));
                }
                // Hash password if applicable
                if field.field_type == FieldType::Password
                    && let Value::String(pass) = val
                {
                    *val = Value::String(hash_password(pass));
                }
            }
        }
        Ok(())
    }

    pub async fn list(
        &self,
        entity: &str,
        limit: Option<usize>,
        offset: Option<usize>,
        filters: Option<std::collections::HashMap<String, String>>,
        ctx: &RuntimeContext,
    ) -> Result<Vec<Arc<Value>>, String> {
        if self.schema.queries.contains_key(&Symbol::from(entity)) {
            // Check permission for query?
            // Generally we check read permission for the root entity of the query.
            // Or a specific permission for the query name.
            // For now, let's look up the query and check its root entity.
            let query = self.schema.queries.get(&Symbol::from(entity)).unwrap(); // Verified by contains_key
            let root_entity = query.root_entity.as_str();

            // We check read permission for the root entity
            if let Some(entity_schema) = self.schema.entities.get(&query.root_entity) {
                let read_perm = entity_schema
                    .options
                    .get("read_permission")
                    .cloned()
                    .unwrap_or_else(|| format!("read:{}", root_entity));
                self.validate_permission(ctx, &read_perm, "read", root_entity)?;
            } else {
                // Query might refer to non-entity? Unlikely.
                // Fallback: check read:{QueryName}
                let read_perm = format!("read:{}", entity);
                self.validate_permission(ctx, &read_perm, "read", entity)?;
            }

            // Recursively check joined entities
            self.check_query_permissions_recursive(&query.joins, ctx)?;

            let mut runtime_params = std::collections::HashMap::new();
            if let Some(f) = filters {
                for (k, v) in f {
                    // Naive conversion: try parse as f64, bool, or keep string
                    if let Ok(b) = v.parse::<bool>() {
                        runtime_params.insert(k, Value::Bool(b));
                    } else if let Ok(f) = v.parse::<f64>() {
                        if let Some(n) = serde_json::Number::from_f64(f) {
                            runtime_params.insert(k, Value::Number(n));
                        } else {
                            runtime_params.insert(k, Value::String(v));
                        }
                    } else {
                        runtime_params.insert(k, Value::String(v));
                    }
                }
            }

            let strategy = QueryEngine::plan(&self.schema, entity, &runtime_params)?;
            match strategy.plans.first().cloned() {
                Some(QueryPlan::ExecuteSql { mut sql, params }) => {
                    if let Some(l) = limit {
                        sql.push_str(&format!(" LIMIT {}", l));
                    }
                    if let Some(o) = offset {
                        sql.push_str(&format!(" OFFSET {}", o));
                    }
                    return self.datastore.query_with_params(&sql, params).await;
                }
                Some(QueryPlan::ExecuteHierarchy {
                    sql,
                    params,
                    structure_sql,
                    structure_params,
                    parent_field,
                    rollup_fields,
                }) => {
                    // 1. Fetch minimal structure (flat)
                    let records = self
                        .datastore
                        .query_with_params(&structure_sql, structure_params)
                        .await
                        .map_err(|e| e.to_string())?;

                    // 2. Build Tree Maps
                    let mut records_map: std::collections::HashMap<String, Arc<Value>> =
                        std::collections::HashMap::new();
                    let mut children_map: std::collections::HashMap<String, Vec<String>> =
                        std::collections::HashMap::new();
                    let mut roots: Vec<String> = Vec::new();

                    for record in &records {
                        if let Some(obj) = record.as_object()
                            && let Some(id) = obj.get("id").and_then(|v| v.as_str())
                        {
                            records_map.insert(id.to_string(), record.clone());

                            let parent_id = obj.get(&parent_field).and_then(|v| v.as_str()).map(|s| s.to_string());

                            if let Some(pid) = parent_id {
                                if !pid.is_empty() {
                                    children_map.entry(pid).or_default().push(id.to_string());
                                } else {
                                    roots.push(id.to_string());
                                }
                            } else {
                                roots.push(id.to_string());
                            }
                        }
                    }

                    // 3. Recursive Rollup & Flatten (returns structure + rollups)
                    let page_results =
                        self.build_hierarchy(&roots, &records_map, &children_map, &rollup_fields, limit, offset)?;

                    if page_results.is_empty() {
                        return Ok(vec![]);
                    }

                    // 4. Fetch Details for visible page
                    let mut ids = Vec::new();
                    for r in &page_results {
                        if let Some(id) = r.get("id").and_then(|v| v.as_str()) {
                            ids.push(id.to_string());
                        }
                    }

                    if ids.is_empty() {
                        return Ok(page_results);
                    }

                    // Wrap original SQL in subquery to filter by ID
                    let mut detail_params = params.clone();
                    let mut placeholders = Vec::new();

                    let is_postgres = self
                        .schema
                        .database
                        .as_ref()
                        .map(|d| d.db_type == gurih_ir::DatabaseType::Postgres)
                        .unwrap_or(false);

                    for (i, id) in ids.iter().enumerate() {
                        detail_params.push(Value::String(id.clone()));
                        if is_postgres {
                            placeholders.push(format!("${}", params.len() + i + 1));
                        } else {
                            placeholders.push("?".to_string());
                        }
                    }

                    let details_sql = format!(
                        "SELECT * FROM ({}) AS details WHERE details.id IN ({})",
                        sql,
                        placeholders.join(", ")
                    );

                    let details = self
                        .datastore
                        .query_with_params(&details_sql, detail_params)
                        .await
                        .map_err(|e| format!("Failed to fetch hierarchy details: {}", e))?;

                    // 5. Merge Details
                    let details_map: std::collections::HashMap<String, Arc<Value>> = details
                        .into_iter()
                        .filter_map(|d| {
                            let id = d.get("id").and_then(|v| v.as_str()).map(|s| s.to_string());
                            id.map(|i| (i, d))
                        })
                        .collect();

                    let mut final_page = Vec::with_capacity(page_results.len());

                    for node in page_results {
                        let id = node.get("id").and_then(|v| v.as_str()).unwrap_or("");

                        // Start with detail object (full fields)
                        let mut merged = if let Some(detail) = details_map.get(id) {
                            detail.as_object().cloned().unwrap_or_default()
                        } else {
                            // Fallback to node (structure only)
                            node.as_object().cloned().unwrap_or_default()
                        };

                        // Overlay structure metadata (rollups, _level, etc)
                        if let Some(node_obj) = node.as_object() {
                            for (k, v) in node_obj {
                                // Overwrite details with computed values (rollups) and metadata
                                merged.insert(k.clone(), v.clone());
                            }
                        }
                        final_page.push(Arc::new(Value::Object(merged)));
                    }

                    return Ok(final_page);
                }
                None => return Err("Query engine failed to produce SQL plan".to_string()),
            }
        }

        if let Some(schema) = self.schema.entities.get(&Symbol::from(entity)) {
            // Validate read permission
            let read_perm = schema
                .options
                .get("read_permission")
                .cloned()
                .unwrap_or_else(|| format!("read:{}", entity));

            self.validate_permission(ctx, &read_perm, "read", entity)?;

            // Sentinel: Fix filter bypass vulnerability
            // Previous implementation ignored filters for entities, potentially exposing restricted data.
            // Now we use `find` if filters are present, with in-memory pagination fallback.
            if let Some(f) = filters {
                if !f.is_empty() {
                    let all_results = self.datastore.find(schema.table_name.as_str(), f).await?;
                    // Manual Pagination (since find doesn't support limit/offset)
                    let skip = offset.unwrap_or(0);
                    if skip >= all_results.len() {
                        return Ok(vec![]);
                    }
                    let take = limit.unwrap_or(usize::MAX);
                    let end = if take == usize::MAX {
                        all_results.len()
                    } else {
                        std::cmp::min(skip.saturating_add(take), all_results.len())
                    };
                    return Ok(all_results[skip..end].to_vec());
                }
            }

            self.datastore.list(schema.table_name.as_str(), limit, offset).await
        } else {
            Err(format!("Entity or Query '{}' not defined", entity))
        }
    }

    fn check_query_permissions_recursive(&self, joins: &[QueryJoin], ctx: &RuntimeContext) -> Result<(), String> {
        for join in joins {
            let target_entity = join.target_entity.as_str();
            if let Some(entity_schema) = self.schema.entities.get(&join.target_entity) {
                let read_perm = entity_schema
                    .options
                    .get("read_permission")
                    .cloned()
                    .unwrap_or_else(|| format!("read:{}", target_entity));

                self.validate_permission(ctx, &read_perm, "read", target_entity)?;
            }

            // Recurse
            self.check_query_permissions_recursive(&join.joins, ctx)?;
        }
        Ok(())
    }

    fn build_hierarchy(
        &self,
        roots: &[String],
        records_map: &std::collections::HashMap<String, Arc<Value>>,
        children_map: &std::collections::HashMap<String, Vec<String>>,
        rollup_fields: &[String],
        limit: Option<usize>,
        offset: Option<usize>,
    ) -> Result<Vec<Arc<Value>>, String> {
        let mut result = Vec::new();

        // 1. Compute rollups (Post-order) -> returns Map<ID, RollupValues>
        let mut rollups_cache = std::collections::HashMap::new();
        let mut visited = std::collections::HashSet::new();

        for root in roots {
            self.compute_rollups(
                root,
                records_map,
                children_map,
                rollup_fields,
                &mut rollups_cache,
                &mut visited,
            )?;
        }

        // 2. Flatten (Pre-order)
        visited.clear();
        for root in roots {
            // group maps into a small context to reduce function arguments
            let ctx = HierarchyContext {
                records_map,
                children_map,
                rollups_cache: &rollups_cache,
            };

            self.flatten_hierarchy(root, 0, &ctx, &mut result, &mut visited)?;
        }

        // 3. Pagination
        let start = offset.unwrap_or(0);
        if start >= result.len() {
            return Ok(vec![]);
        }
        let end = limit.map(|l| start + l).unwrap_or(result.len());
        let end = std::cmp::min(end, result.len());

        Ok(result[start..end].to_vec())
    }

    fn compute_rollups(
        &self,
        id: &str,
        records_map: &std::collections::HashMap<String, Arc<Value>>,
        children_map: &std::collections::HashMap<String, Vec<String>>,
        rollup_fields: &[String],
        cache: &mut std::collections::HashMap<String, serde_json::Map<String, Value>>,
        visited: &mut std::collections::HashSet<String>,
    ) -> Result<serde_json::Map<String, Value>, String> {
        if visited.contains(id) {
            return Err(format!("Cycle detected in hierarchy at id: {}", id));
        }
        visited.insert(id.to_string());

        if let Some(res) = cache.get(id) {
            visited.remove(id);
            return Ok(res.clone());
        }

        let record = records_map.get(id).ok_or("Record not found")?;
        let obj = record.as_object().cloned().unwrap_or_default();

        let mut current_rollup = serde_json::Map::new();
        let parse_f64 = |v: &Value| match v {
            Value::Number(n) => n.as_f64().unwrap_or(0.0),
            Value::String(s) => s.parse().unwrap_or(0.0),
            _ => 0.0,
        };

        for field in rollup_fields {
            let val = obj.get(field).map(parse_f64).unwrap_or(0.0);
            current_rollup.insert(field.clone(), Value::from(val));
        }

        if let Some(children) = children_map.get(id) {
            for child in children {
                let child_vals =
                    self.compute_rollups(child, records_map, children_map, rollup_fields, cache, visited)?;
                for field in rollup_fields {
                    let cur = current_rollup.get(field).and_then(|v| v.as_f64()).unwrap_or(0.0);
                    let child = child_vals.get(field).and_then(|v| v.as_f64()).unwrap_or(0.0);
                    current_rollup.insert(field.clone(), Value::from(cur + child));
                }
            }
        }

        cache.insert(id.to_string(), current_rollup.clone());
        visited.remove(id);
        Ok(current_rollup)
    }

    fn flatten_hierarchy(
        &self,
        id: &str,
        level: usize,
        ctx: &HierarchyContext,
        result: &mut Vec<Arc<Value>>,
        visited: &mut std::collections::HashSet<String>,
    ) -> Result<(), String> {
        if visited.contains(id) {
            return Err(format!("Cycle detected in hierarchy at id: {}", id));
        }
        visited.insert(id.to_string());

        let record = ctx.records_map.get(id).ok_or("Record not found")?;
        let mut obj = record.as_object().cloned().unwrap_or_default();

        // Apply rollups
        if let Some(rollup) = ctx.rollups_cache.get(id) {
            for (k, v) in rollup {
                obj.insert(k.clone(), v.clone());
            }
        }

        let children = ctx.children_map.get(id);
        let is_leaf = children.is_none() || children.unwrap().is_empty();

        obj.insert("_level".to_string(), Value::from(level));
        obj.insert("_is_leaf".to_string(), Value::Bool(is_leaf));
        obj.insert("_has_children".to_string(), Value::Bool(!is_leaf));

        result.push(Arc::new(Value::Object(obj)));

        if let Some(kids) = children {
            for kid in kids {
                self.flatten_hierarchy(kid, level + 1, ctx, result, visited)?;
            }
        }
        visited.remove(id);
        Ok(())
    }

    async fn execute_posting_rule(
        &self,
        rule_name: &Symbol,
        source_data: &Value,
        ctx: &RuntimeContext,
    ) -> Result<(), String> {
        let rule = self
            .schema
            .posting_rules
            .get(rule_name)
            .ok_or_else(|| format!("Posting rule '{}' not found", rule_name))?;

        // Prepare Context with "doc"
        let mut context_map = serde_json::Map::new();
        context_map.insert("doc".to_string(), source_data.clone());
        let context = Value::Object(context_map);

        // Evaluate Description
        let description = crate::evaluator::evaluate(
            &rule.description_expr,
            &context,
            Some(&self.schema),
            Some(&self.datastore),
        )
        .await
        .map_err(|e| format!("Failed to evaluate description: {}", e))?
        .as_str()
        .unwrap_or("")
        .to_string();

        // Evaluate Date
        let date_val = crate::evaluator::evaluate(&rule.date_expr, &context, Some(&self.schema), Some(&self.datastore))
            .await
            .map_err(|e| format!("Failed to evaluate date: {}", e))?;

        let date_str = date_val.as_str().unwrap_or("").to_string();

        let mut journal_lines = vec![];

        let mut unique_terms = std::collections::HashSet::new();
        for line in &rule.lines {
            unique_terms.insert(line.account.as_str());
        }

        let account_table = self
            .schema
            .entities
            .get(&Symbol::from("Account"))
            .map(|e| e.table_name.as_str())
            .unwrap_or("Account");

        let has_system_tag = self
            .schema
            .entities
            .get(&Symbol::from("Account"))
            .map(|e| e.fields.iter().any(|f| f.name == Symbol::from("system_tag")))
            .unwrap_or(false);

        let mut code_map = HashMap::new();
        let mut name_map = HashMap::new();
        let mut tag_map = HashMap::new();

        // Batch Fetch if DB is configured
        if let Some(db_config) = &self.schema.database {
            let terms_vec: Vec<&str> = unique_terms.iter().cloned().collect();
            // Chunk size to respect DB parameter limits (safe margin)
            let chunks = terms_vec.chunks(500);

            for chunk in chunks {
                let mut params = Vec::new();
                let mut placeholders_code = Vec::new();
                let mut placeholders_name = Vec::new();
                let mut placeholders_tag = Vec::new();

                for (i, term) in chunk.iter().enumerate() {
                    params.push(Value::String(term.to_string()));
                    placeholders_code.push(gurih_ir::utils::get_db_placeholder(&db_config.db_type, i + 1));
                }

                // For name/tag clause placeholders
                if db_config.db_type == gurih_ir::DatabaseType::Postgres {
                    // Postgres can reuse params $1..$N
                    for i in 0..chunk.len() {
                        let p = format!("${}", i + 1);
                        placeholders_name.push(p.clone());
                        if has_system_tag {
                            placeholders_tag.push(p);
                        }
                    }
                } else {
                    // SQLite needs params to be repeated
                    for term in chunk {
                        params.push(Value::String(term.to_string()));
                    }
                    for _ in 0..chunk.len() {
                        placeholders_name.push("?".to_string());
                    }
                    if has_system_tag {
                        for term in chunk {
                            params.push(Value::String(term.to_string()));
                        }
                        for _ in 0..chunk.len() {
                            placeholders_tag.push("?".to_string());
                        }
                    }
                }

                let cols = if has_system_tag {
                    "id, code, name, system_tag"
                } else {
                    "id, code, name"
                };

                let mut where_clause = format!(
                    "code IN ({}) OR name IN ({})",
                    placeholders_code.join(", "),
                    placeholders_name.join(", ")
                );

                if has_system_tag {
                    where_clause.push_str(&format!(" OR system_tag IN ({})", placeholders_tag.join(", ")));
                }

                let sql = format!("SELECT {} FROM {} WHERE {}", cols, account_table, where_clause);

                if let Ok(rows) = self.datastore.query_with_params(&sql, params).await {
                    for row in rows {
                        if let Some(obj) = row.as_object() {
                            let id = obj.get("id").and_then(|v| v.as_str()).unwrap_or_default().to_string();
                            let code = obj.get("code").and_then(|v| v.as_str()).unwrap_or_default().to_string();
                            let name = obj.get("name").and_then(|v| v.as_str()).unwrap_or_default().to_string();

                            if !code.is_empty() {
                                code_map.insert(code, id.clone());
                            }
                            if !name.is_empty() {
                                name_map.insert(name, id.clone());
                            }
                            if has_system_tag
                                && let Some(tag) = obj.get("system_tag").and_then(|v| v.as_str())
                                && !tag.is_empty()
                            {
                                tag_map.insert(tag.to_string(), id);
                            }
                        }
                    }
                }
            }
        } else {
            // Parallel Fetch Optimization for non-SQL stores
            let terms_vec: Vec<&str> = unique_terms.iter().cloned().collect();

            // 1. Find by Code
            let mut futures = Vec::new();
            for term in &terms_vec {
                let mut filters = HashMap::new();
                filters.insert("code".to_string(), term.to_string());
                futures.push(self.datastore.find_first(account_table, filters));
            }

            let results = join_all(futures).await;
            for (i, result) in results.into_iter().enumerate() {
                if let Ok(Some(account)) = result
                    && let Some(obj) = account.as_object()
                {
                    let id = obj.get("id").and_then(|v| v.as_str()).unwrap_or_default().to_string();
                    code_map.insert(terms_vec[i].to_string(), id);
                }
            }

            // 2. Find by System Tag (if applicable)
            if has_system_tag {
                let mut tag_futures = Vec::new();
                let mut pending_terms_tag = Vec::new();

                for term in &terms_vec {
                    if !code_map.contains_key(*term) {
                        let mut filters = HashMap::new();
                        filters.insert("system_tag".to_string(), term.to_string());
                        tag_futures.push(self.datastore.find_first(account_table, filters));
                        pending_terms_tag.push(*term);
                    }
                }

                if !tag_futures.is_empty() {
                    let results = join_all(tag_futures).await;
                    for (i, result) in results.into_iter().enumerate() {
                        if let Ok(Some(account)) = result
                            && let Some(obj) = account.as_object()
                        {
                            let id = obj.get("id").and_then(|v| v.as_str()).unwrap_or_default().to_string();
                            tag_map.insert(pending_terms_tag[i].to_string(), id);
                        }
                    }
                }
            }

            // 3. Find by Name (last resort)
            let mut name_futures = Vec::new();
            let mut pending_terms_name = Vec::new();

            for term in &terms_vec {
                if !code_map.contains_key(*term) && !tag_map.contains_key(*term) {
                    let mut filters = HashMap::new();
                    filters.insert("name".to_string(), term.to_string());
                    name_futures.push(self.datastore.find_first(account_table, filters));
                    pending_terms_name.push(*term);
                }
            }

            if !name_futures.is_empty() {
                let results = join_all(name_futures).await;
                for (i, result) in results.into_iter().enumerate() {
                    if let Ok(Some(account)) = result
                        && let Some(obj) = account.as_object()
                    {
                        let id = obj.get("id").and_then(|v| v.as_str()).unwrap_or_default().to_string();
                        name_map.insert(pending_terms_name[i].to_string(), id);
                    }
                }
            }
        }

        for line in &rule.lines {
            let mut line_obj = serde_json::Map::new();

            // Resolve Account (Priority: System Tag > Code > Name)
            let account_term = line.account.as_str();

            let account_id = if let Some(id) = tag_map.get(account_term) {
                id.clone()
            } else if let Some(id) = code_map.get(account_term) {
                id.clone()
            } else if let Some(id) = name_map.get(account_term) {
                id.clone()
            } else {
                // Fallback to individual find
                let mut found_id = None;

                if has_system_tag {
                    let mut filters = HashMap::new();
                    filters.insert("system_tag".to_string(), account_term.to_string());
                    if let Ok(Some(acc)) = self.datastore.find_first(account_table, filters).await {
                        found_id = acc.get("id").and_then(|v| v.as_str()).map(|s| s.to_string());
                    }
                }

                if found_id.is_none() {
                    let mut filters = HashMap::new();
                    filters.insert("code".to_string(), account_term.to_string());
                    if let Ok(Some(acc)) = self.datastore.find_first(account_table, filters).await {
                        found_id = acc.get("id").and_then(|v| v.as_str()).map(|s| s.to_string());
                    }
                }

                if found_id.is_none() {
                    let mut filters = HashMap::new();
                    filters.insert("name".to_string(), account_term.to_string());
                    if let Ok(Some(acc)) = self.datastore.find_first(account_table, filters).await {
                        found_id = acc.get("id").and_then(|v| v.as_str()).map(|s| s.to_string());
                    }
                }

                found_id.ok_or_else(|| format!("Account '{}' not found", account_term))?
            };

            line_obj.insert("account".to_string(), Value::String(account_id));

            // Helper to ensure Money fields are strings
            let to_money_val = |v: Value| -> Value {
                match v {
                    Value::Number(n) => Value::String(n.to_string()),
                    Value::String(s) => Value::String(s),
                    _ => Value::String("0.00".to_string()),
                }
            };

            if let Some(debit_expr) = &line.debit_expr {
                let val = crate::evaluator::evaluate(debit_expr, &context, Some(&self.schema), Some(&self.datastore))
                    .await
                    .map_err(|e| format!("Failed to evaluate debit: {}", e))?;

                line_obj.insert("debit".to_string(), to_money_val(val));
                line_obj.insert("credit".to_string(), Value::String("0.00".to_string()));
            } else if let Some(credit_expr) = &line.credit_expr {
                let val = crate::evaluator::evaluate(credit_expr, &context, Some(&self.schema), Some(&self.datastore))
                    .await
                    .map_err(|e| format!("Failed to evaluate credit: {}", e))?;

                line_obj.insert("credit".to_string(), to_money_val(val));
                line_obj.insert("debit".to_string(), Value::String("0.00".to_string()));
            }

            // Process arbitrary fields
            for (field_name, expr) in &line.fields {
                let val = crate::evaluator::evaluate(expr, &context, Some(&self.schema), Some(&self.datastore))
                    .await
                    .map_err(|e| format!("Failed to evaluate field '{}': {}", field_name, e))?;
                line_obj.insert(field_name.to_string(), val);
            }

            journal_lines.push(Value::Object(line_obj));
        }

        // Create JournalEntry Header
        let mut journal = serde_json::Map::new();
        journal.insert("description".to_string(), Value::String(description));
        journal.insert("date".to_string(), Value::String(date_str));
        journal.insert("status".to_string(), Value::String("Draft".to_string()));
        // Note: We DO NOT insert "lines" here anymore.

        let journal_id = self.create("JournalEntry", Value::Object(journal), ctx).await?;

        // Create Journal Lines
        for line_val in &mut journal_lines {
            if let Some(obj) = line_val.as_object_mut() {
                obj.insert("journal_entry".to_string(), Value::String(journal_id.clone()));
            }
        }

        // Use create_many() to ensure validation logic runs efficiently in batch
        self.create_many("JournalLine", journal_lines, ctx).await?;

        Ok(())
    }
}
