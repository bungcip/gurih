use crate::context::RuntimeContext;
use crate::datastore::DataStore;
use crate::errors::RuntimeError;
use crate::traits::DataAccess;
use async_trait::async_trait;
use gurih_ir::{ActionStep, Expression, Schema, Symbol};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;

#[async_trait]
pub trait Plugin: Send + Sync {
    fn name(&self) -> &str;

    /// Checks a custom precondition.
    async fn check_precondition(
        &self,
        name: &str,
        args: &[Expression],
        kwargs: &HashMap<String, String>,
        entity_data: &Value,
        schema: &Schema,
        datastore: Option<&Arc<dyn DataStore>>,
    ) -> Result<(), RuntimeError>;

    /// Applies a custom effect.
    #[allow(clippy::too_many_arguments)]
    async fn apply_effect(
        &self,
        name: &str,
        args: &[Expression],
        kwargs: &HashMap<String, String>,
        schema: &Schema,
        datastore: Option<&Arc<dyn DataStore>>,
        entity_name: &str,
        entity_data: &Value,
    ) -> Result<(Value, Vec<String>, Vec<Symbol>), RuntimeError>;

    /// Executes a custom action step.
    /// Returns Ok(true) if handled, Ok(false) if not recognized.
    async fn execute_action_step(
        &self,
        step_name: &str,
        step: &ActionStep,
        params: &HashMap<String, String>,
        data_access: &dyn DataAccess,
        ctx: &RuntimeContext,
    ) -> Result<bool, RuntimeError>;
}
