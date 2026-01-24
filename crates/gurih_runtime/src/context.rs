use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeContext {
    pub user_id: String,
    pub roles: Vec<String>,
    pub permissions: Vec<String>,
    pub token: Option<String>,
}

impl RuntimeContext {
    pub fn system() -> Self {
        Self {
            user_id: "system".to_string(),
            roles: vec!["admin".to_string()],
            permissions: vec!["*".to_string()],
            token: None,
        }
    }

    pub fn has_permission(&self, permission: &str) -> bool {
        self.permissions.contains(&"*".to_string()) || self.permissions.contains(&permission.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_runtime_context_permissions() {
        // Case 1: System context has all permissions
        let sys = RuntimeContext::system();
        assert!(sys.has_permission("any_permission"));

        // Case 2: Specific permissions
        let user = RuntimeContext {
            user_id: "user".to_string(),
            roles: vec!["user".to_string()],
            permissions: vec!["read".to_string(), "write".to_string()],
            token: None,
        };

        assert!(user.has_permission("read"));
        assert!(user.has_permission("write"));
        assert!(!user.has_permission("delete"));
    }
}
