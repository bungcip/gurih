use gurih_ir::Schema;

pub struct WorkflowEngine;

impl Default for WorkflowEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl WorkflowEngine {
    pub fn new() -> Self {
        Self
    }

    pub fn validate_transition(
        &self,
        schema: &Schema,
        entity_name: &str,
        current_state: &str,
        new_state: &str,
    ) -> Result<(), String> {
        // Find workflow for entity
        let workflow = schema.workflows.values().find(|w| w.entity == entity_name);

        if let Some(wf) = workflow {
            // If staying in same state, it's usually allowed (update other fields)
            if current_state == new_state {
                return Ok(());
            }

            // Check if transition exists from current to new
            let valid = wf
                .transitions
                .iter()
                .any(|t| t.from == current_state && t.to == new_state);
            if !valid {
                return Err(format!(
                    "Invalid transition from '{}' to '{}' for entity '{}'",
                    current_state, new_state, entity_name
                ));
            }
        }

        Ok(())
    }

    pub fn get_initial_state(&self, schema: &Schema, entity_name: &str) -> Option<String> {
        schema
            .workflows
            .values()
            .find(|w| w.entity == entity_name)
            .map(|w| w.initial_state.clone())
    }

    pub fn get_transition_permission(
        &self,
        schema: &Schema,
        entity_name: &str,
        current_state: &str,
        new_state: &str,
    ) -> Option<String> {
        if current_state == new_state {
            return None;
        }

        let workflow = schema
            .workflows
            .values()
            .find(|w| w.entity == entity_name)?;
        workflow
            .transitions
            .iter()
            .find(|t| t.from == current_state && t.to == new_state)
            .and_then(|t| t.required_permission.clone())
    }
}
