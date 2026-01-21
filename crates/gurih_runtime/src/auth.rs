use crate::context::RuntimeContext;
use crate::datastore::DataStore;
use std::collections::HashMap;
use std::sync::Arc;

pub struct AuthEngine {
    datastore: Arc<dyn DataStore>,
}

impl AuthEngine {
    pub fn new(datastore: Arc<dyn DataStore>) -> Self {
        Self { datastore }
    }

    pub async fn login(&self, username: &str, password: &str) -> Result<RuntimeContext, String> {
        let mut filters = HashMap::new();
        filters.insert("username".to_string(), username.to_string());

        let users = self.datastore.find("User", filters).await?;
        if users.is_empty() {
            return Err("Invalid username or password".to_string());
        }

        let user = &users[0];
        let stored_password = user.get("password").and_then(|v| v.as_str()).unwrap_or_default();

        if stored_password != password {
            return Err("Invalid username or password".to_string());
        }

        let user_id = user.get("id").and_then(|v| v.as_str()).unwrap_or_default().to_string();

        let role = user.get("role").and_then(|v| v.as_str()).unwrap_or("user").to_string();

        // Map role to permissions (simplified for now)
        let permissions = if role == "admin" || role == "HRManager" {
            vec!["*".to_string()]
        } else {
            vec![]
        };

        Ok(RuntimeContext {
            user_id,
            roles: vec![role],
            permissions,
        })
    }
}
