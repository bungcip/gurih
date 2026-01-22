use crate::context::RuntimeContext;
use crate::datastore::DataStore;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;

pub struct AuthEngine {
    datastore: Arc<dyn DataStore>,
}

pub fn hash_password(password: &str) -> String {
    let salt = Uuid::new_v4().to_string();
    let mut hasher = Sha256::new();
    hasher.update(salt.as_bytes());
    hasher.update(password.as_bytes());
    let hash = hex::encode(hasher.finalize());
    format!("{}${}", salt, hash)
}

pub fn verify_password(password: &str, stored_value: &str) -> bool {
    let parts: Vec<&str> = stored_value.split('$').collect();
    if parts.len() != 2 {
        return false;
    }
    let salt = parts[0];
    let hash = parts[1];

    let mut hasher = Sha256::new();
    hasher.update(salt.as_bytes());
    hasher.update(password.as_bytes());
    let computed_hash = hex::encode(hasher.finalize());

    computed_hash == hash
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

        if !verify_password(password, stored_password) {
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
