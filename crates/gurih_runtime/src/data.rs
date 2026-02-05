use crate::auth::hash_password;
use crate::context::RuntimeContext;
use crate::datastore::DataStore;
use crate::plugins::Plugin;
use crate::query_engine::{QueryEngine, QueryPlan};
use crate::traits::DataAccess;
use crate::workflow::WorkflowEngine;
use async_trait::async_trait;
use gurih_ir::{DatabaseType, FieldType, Schema, Symbol};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;

pub struct DataEngine {
    schema: Arc<Schema>,
    datastore: Arc<dyn DataStore>,
    workflow: WorkflowEngine,
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

    async fn read(&self, entity_name: &str, id: &str) -> Result<Option<Arc<Value>>, String> {
        self.read(entity_name, id).await
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
    ) -> Result<Vec<Arc<Value>>, String> {
        self.list(entity, limit, offset, filters).await
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

        self.validate_permission(ctx, &create_perm, entity_name)?;

        // Rule Check (Create)
        self.check_rules(entity_name, "create", &data, None).await?;

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

    pub async fn read(&self, entity_name: &str, id: &str) -> Result<Option<Arc<Value>>, String> {
        let entity_schema = self.schema.entities.get(&Symbol::from(entity_name));
        if let Some(schema) = entity_schema {
            self.datastore.get(schema.table_name.as_str(), id).await
        } else {
            Err(format!("Entity '{}' not defined", entity_name))
        }
    }

    pub async fn update(&self, entity_name: &str, id: &str, data: Value, ctx: &RuntimeContext) -> Result<(), String> {
        let entity_schema = self
            .schema
            .entities
            .get(&Symbol::from(entity_name))
            .ok_or_else(|| format!("Entity '{}' not defined", entity_name))?;

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

        let mut current_record_opt: Option<Arc<Value>> = None;

        if workflow.is_some() || has_update_rules || track_changes {
            current_record_opt = self.datastore.get(entity_schema.table_name.as_str(), id).await?;
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
                    .apply_effects(&self.schema, entity_name, current_state, new_state, &merged_record)
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

        let mut current_record_opt: Option<Arc<Value>> = None;

        if workflow.is_some() || has_delete_rules {
            current_record_opt = self.read(entity_name, id).await?;
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

    fn validate_permission(&self, ctx: &RuntimeContext, permission: &str, entity_name: &str) -> Result<(), String> {
        if !ctx.has_permission(permission) {
            Err(format!(
                "Missing permission '{}' to create entity '{}'",
                permission, entity_name
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
    ) -> Result<Vec<Arc<Value>>, String> {
        if self.schema.queries.contains_key(&Symbol::from(entity)) {
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
            if let Some(QueryPlan::ExecuteSql { mut sql, params }) = strategy.plans.first().cloned() {
                if let Some(l) = limit {
                    sql.push_str(&format!(" LIMIT {}", l));
                }
                if let Some(o) = offset {
                    sql.push_str(&format!(" OFFSET {}", o));
                }
                return self.datastore.query_with_params(&sql, params).await;
            }
            return Err("Query engine failed to produce SQL plan".to_string());
        }

        if let Some(schema) = self.schema.entities.get(&Symbol::from(entity)) {
            self.datastore.list(schema.table_name.as_str(), limit, offset).await
        } else {
            Err(format!("Entity or Query '{}' not defined", entity))
        }
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
        for line in &rule.lines {
            let mut line_obj = serde_json::Map::new();

            // Resolve Account (Simple Lookup)
            let account_term = line.account.as_str();

            // Try Find by Code
            let mut filters = HashMap::new();
            filters.insert("code".to_string(), account_term.to_string());

            // Note: We use the Entity name "Account" to resolve table name internally in find/list
            // assuming datastore handles entity->table mapping or we pass table name.
            // DataStore trait takes 'entity' (which is often table name in implementation).
            // But DataEngine usually resolves it.
            // Here we should use the schema to get table name.
            let account_table = self
                .schema
                .entities
                .get(&Symbol::from("Account"))
                .map(|e| e.table_name.as_str())
                .unwrap_or("Account");

            let mut accounts = self
                .datastore
                .find(account_table, filters)
                .await
                .map_err(|e| e.to_string())?;

            if accounts.is_empty() {
                // Try Find by Name
                let mut filters = HashMap::new();
                filters.insert("name".to_string(), account_term.to_string());
                accounts = self
                    .datastore
                    .find(account_table, filters)
                    .await
                    .map_err(|e| e.to_string())?;
            }

            let account_id = accounts
                .first()
                .and_then(|row| row.get("id").and_then(|v| v.as_str()))
                .ok_or_else(|| format!("Account '{}' not found", account_term))?;

            line_obj.insert("account".to_string(), Value::String(account_id.to_string()));

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
        for mut line_val in journal_lines {
            if let Some(obj) = line_val.as_object_mut() {
                obj.insert("journal_entry".to_string(), Value::String(journal_id.clone()));
            }
            // Use create() to ensure validation logic runs
            self.create("JournalLine", line_val, ctx).await?;
        }

        Ok(())
    }
}
