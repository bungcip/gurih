use async_trait::async_trait;
use gurih_ir::{ActionStep, Expression, Schema, Symbol};
use gurih_runtime::context::RuntimeContext;
use gurih_runtime::datastore::DataStore;
use gurih_runtime::errors::RuntimeError;
use gurih_runtime::plugins::Plugin;
use gurih_runtime::traits::DataAccess;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;

pub mod actions;
pub mod effects;
pub mod utils;
pub mod validation;

use actions::{execute_generate_closing_entry, execute_reverse_journal};
use effects::execute_snapshot_parties;
use validation::{check_balanced_transaction, check_period_open, check_valid_parties};

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
