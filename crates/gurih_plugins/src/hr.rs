use gurih_runtime::context::RuntimeContext;
use gurih_runtime::datastore::DataStore;
use gurih_runtime::errors::RuntimeError;
use gurih_runtime::plugins::Plugin;
use gurih_runtime::traits::DataAccess;
use async_trait::async_trait;
use gurih_ir::{ActionStep, Expression, Schema, Symbol};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;

pub struct HrPlugin;

#[async_trait]
impl Plugin for HrPlugin {
    fn name(&self) -> &str {
        "HrPlugin"
    }

    async fn check_precondition(
        &self,
        _name: &str,
        _args: &[Expression],
        _entity_data: &Value,
        _schema: &Schema,
        _datastore: Option<&Arc<dyn DataStore>>,
    ) -> Result<(), RuntimeError> {
        // Placeholder for future custom checks
        Ok(())
    }

    async fn apply_effect(
        &self,
        name: &str,
        args: &[Expression],
        _schema: &Schema,
        _entity_name: &str,
        _entity_data: &Value,
    ) -> Result<(Value, Vec<String>, Vec<Symbol>), RuntimeError> {
        match name {
            "suspend_payroll" => {
                let suspend = if let Some(Expression::StringLiteral(s)) = args.first() {
                    s == "true"
                } else {
                    true
                };

                let mut updates = serde_json::Map::new();
                updates.insert("is_payroll_active".to_string(), Value::Bool(!suspend));

                Ok((Value::Object(updates), vec![], vec![]))
            }
            "update_rank_eligibility" => {
                let eligible = if let Some(Expression::StringLiteral(s)) = args.first() {
                    s == "true"
                } else {
                    true
                };

                let mut updates = serde_json::Map::new();
                updates.insert("rank_eligible".to_string(), Value::Bool(eligible));

                Ok((Value::Object(updates), vec![], vec![]))
            }
            _ => Ok((Value::Null, vec![], vec![])),
        }
    }

    async fn execute_action_step(
        &self,
        _step_name: &str,
        _step: &ActionStep,
        _params: &HashMap<String, String>,
        _data_access: &dyn DataAccess,
        _ctx: &RuntimeContext,
    ) -> Result<bool, RuntimeError> {
        Ok(false)
    }
}
