use crate::context::RuntimeContext;
use crate::datastore::DataStore;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use uuid::Uuid;

pub struct AuthEngine {
    datastore: Arc<dyn DataStore>,
    sessions: Arc<Mutex<HashMap<String, RuntimeContext>>>,
    // Track failed attempts: Username -> (Count, Timestamp of first failure in window)
    login_attempts: Arc<Mutex<HashMap<String, (u32, Instant)>>>,
    user_table: String,
}

pub fn hash_password(password: &str) -> String {
    let salt = Uuid::new_v4().to_string();

    // Initial salt mix
    let mut hasher = Sha256::new();
    hasher.update(salt.as_bytes());
    hasher.update(password.as_bytes());
    let mut hash = hasher.finalize();

    // Iterative hashing (PBKDF2-like) - 100,000 rounds
    for _ in 0..100_000 {
        let mut hasher = Sha256::new();
        hasher.update(hash);
        hash = hasher.finalize();
    }

    let hash_hex = hex::encode(hash);
    format!("v2${}${}", salt, hash_hex)
}

pub fn verify_password(password: &str, stored_value: &str) -> bool {
    if !stored_value.starts_with("v2$") {
        return false;
    }

    let parts: Vec<&str> = stored_value.split('$').collect();
    if parts.len() != 3 {
        return false;
    }
    let salt = parts[1];
    let hash_str = parts[2];

    // Initial salt mix
    let mut hasher = Sha256::new();
    hasher.update(salt.as_bytes());
    hasher.update(password.as_bytes());
    let mut current_hash = hasher.finalize();

    for _ in 0..100_000 {
        let mut hasher = Sha256::new();
        hasher.update(current_hash);
        current_hash = hasher.finalize();
    }

    let computed_hash = hex::encode(current_hash);
    computed_hash == hash_str
}

impl AuthEngine {
    pub fn new(datastore: Arc<dyn DataStore>, user_table: Option<String>) -> Self {
        Self {
            datastore,
            sessions: Arc::new(Mutex::new(HashMap::new())),
            login_attempts: Arc::new(Mutex::new(HashMap::new())),
            user_table: user_table.unwrap_or_else(|| "User".to_string()),
        }
    }

    pub async fn login(&self, username: &str, password: &str) -> Result<RuntimeContext, String> {
        // Rate Limiting Check
        {
            let mut attempts = self.login_attempts.lock().unwrap();
            // Copy values to avoid borrow conflict
            if let Some((count, last_time)) = attempts.get(username).copied() {
                if count >= 5 {
                    if last_time.elapsed() < Duration::from_secs(300) {
                        return Err("Too many failed attempts. Please try again later.".to_string());
                    }
                    attempts.remove(username);
                }
            }
        }

        let mut filters = HashMap::new();
        filters.insert("username".to_string(), username.to_string());

        let users = self.datastore.find(&self.user_table, filters).await?;

        // Determine if login is successful
        let mut login_success = false;
        let mut user_ref = None;

        if !users.is_empty() {
            let user = &users[0];
            let stored_password = user.get("password").and_then(|v| v.as_str()).unwrap_or_default();
            if verify_password(password, stored_password) {
                login_success = true;
                user_ref = Some(user);
            }
        }

        if !login_success {
            let mut attempts = self.login_attempts.lock().unwrap();
            let entry = attempts.entry(username.to_string()).or_insert((0, Instant::now()));

            if entry.1.elapsed() > Duration::from_secs(300) {
                // Window expired, reset
                entry.0 = 1;
                entry.1 = Instant::now();
            } else {
                entry.0 += 1;
            }

            return Err("Invalid username or password".to_string());
        }

        // Success - Clear attempts
        {
            let mut attempts = self.login_attempts.lock().unwrap();
            attempts.remove(username);
        }

        let user = user_ref.unwrap();
        let user_id = user.get("id").and_then(|v| v.as_str()).unwrap_or_default().to_string();

        let role = user.get("role").and_then(|v| v.as_str()).unwrap_or("user").to_string();

        // Map role to permissions (simplified for now)
        let permissions = if role == "admin" || role == "HRManager" {
            vec!["*".to_string()]
        } else {
            vec![]
        };

        let token = Uuid::new_v4().to_string();
        let ctx = RuntimeContext {
            user_id,
            roles: vec![role],
            permissions,
            token: Some(token.clone()),
        };

        // Store session
        self.sessions.lock().unwrap().insert(token, ctx.clone());

        Ok(ctx)
    }

    pub fn verify_token(&self, token: &str) -> Option<RuntimeContext> {
        self.sessions.lock().unwrap().get(token).cloned()
    }
}
