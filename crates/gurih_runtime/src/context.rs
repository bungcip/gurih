use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeContext {
    pub user_id: String,
    pub roles: Vec<String>,
    pub permissions: Vec<String>,
}

impl RuntimeContext {
    pub fn system() -> Self {
        Self {
            user_id: "system".to_string(),
            roles: vec!["admin".to_string()],
            permissions: vec!["*".to_string()],
        }
    }

    pub fn has_permission(&self, permission: &str) -> bool {
        self.permissions.contains(&"*".to_string())
            || self.permissions.contains(&permission.to_string())
    }
}
