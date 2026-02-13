use crate::context::RuntimeContext;
use crate::datastore::DataStore;
use hmac::Hmac;
use pbkdf2::pbkdf2;
use serde_json::Value;
use sha2::Sha256;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use subtle::ConstantTimeEq;
use uuid::Uuid;

struct Session {
    ctx: RuntimeContext,
    expires_at: Instant,
}

pub struct AuthEngine {
    datastore: Arc<dyn DataStore>,
    sessions: Arc<Mutex<HashMap<String, Session>>>,
    // Track failed attempts: Username -> (Count, Timestamp of first failure in window)
    login_attempts: Arc<Mutex<HashMap<String, (u32, Instant)>>>,
    user_table: String,
    dummy_hash: String,
}

// Sentinel: Limits for preventing memory exhaustion
const MAX_LOGIN_ATTEMPTS: usize = 1000;
const MAX_SESSIONS: usize = 10000;

pub fn hash_password(password: &str) -> String {
    // Generate a random salt
    let salt = Uuid::new_v4().to_string();
    // V4 uses 600,000 iterations (OWASP recommendation for PBKDF2-HMAC-SHA256)
    let iterations = 600_000;

    let mut hash = [0u8; 32];
    pbkdf2::<Hmac<Sha256>>(password.as_bytes(), salt.as_bytes(), iterations, &mut hash).expect("HMAC error");

    let hash_hex = hex::encode(hash);

    // Format: v4$iterations$salt$hash
    format!("v4${}${}${}", iterations, salt, hash_hex)
}

fn verify_password(password: &str, stored_value: &str) -> bool {
    let mut valid_format = false;
    let mut salt = "dummy_salt";
    let mut stored_hash_str = "";
    // Default iterations for verification flow (to prevent timing attacks by always running PBKDF2)
    // We default to v4 cost (600k) if format is invalid, to match dummy_hash cost.
    let mut iterations = 600_000;

    // Parse stored value
    if stored_value.starts_with("v4$") {
        let parts: Vec<&str> = stored_value.split('$').collect();
        if parts.len() == 4 {
            if let Ok(iter) = parts[1].parse::<u32>() {
                iterations = iter;
                salt = parts[2];
                stored_hash_str = parts[3];
                valid_format = true;
            }
        }
    } else if stored_value.starts_with("v3$") {
        // Backward compatibility for v3 (100k iterations, fixed format)
        let parts: Vec<&str> = stored_value.split('$').collect();
        if parts.len() == 3 {
            iterations = 100_000;
            salt = parts[1];
            stored_hash_str = parts[2];
            valid_format = true;
        }
    }

    // Always perform PBKDF2 to prevent timing attacks
    // If format is invalid, we calculate hash with dummy salt (using default 600k cost) and discard result
    let mut computed = [0u8; 32];
    pbkdf2::<Hmac<Sha256>>(password.as_bytes(), salt.as_bytes(), iterations, &mut computed).expect("HMAC error");

    if !valid_format {
        return false;
    }

    let mut stored_hash_bytes = [0u8; 32];
    if hex::decode_to_slice(stored_hash_str, &mut stored_hash_bytes).is_err() {
        return false;
    }

    // Constant-time comparison
    computed.ct_eq(&stored_hash_bytes).into()
}

impl AuthEngine {
    pub fn new(datastore: Arc<dyn DataStore>, user_table: Option<String>) -> Self {
        // Use a random password for dummy hash so it matches nothing known
        // This will now use v4 format (600k iterations)
        let dummy_hash = hash_password(&Uuid::new_v4().to_string());
        Self {
            datastore,
            sessions: Arc::new(Mutex::new(HashMap::new())),
            login_attempts: Arc::new(Mutex::new(HashMap::new())),
            user_table: user_table.unwrap_or_else(|| "user".to_string()),
            dummy_hash,
        }
    }

    #[allow(clippy::collapsible_if)]
    pub async fn login(&self, username: &str, password: &str) -> Result<RuntimeContext, String> {
        self.cleanup_login_attempts();
        self.cleanup_sessions();

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

        let user_opt = self.datastore.find_first(&self.user_table, filters).await?;

        // Determine if login is successful
        let mut stored_password_owned = self.dummy_hash.clone();
        let mut user_ref: Option<Arc<Value>> = None;

        if let Some(user_arc) = user_opt {
            if let Some(pwd) = user_arc.get("password").and_then(|v| v.as_str()) {
                stored_password_owned = pwd.to_string();
            }
            user_ref = Some(user_arc.clone());
        }

        // Always verify password to prevent timing attacks
        let password_valid = verify_password(password, &stored_password_owned);

        if user_ref.is_none() || !password_valid {
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
        self.sessions.lock().unwrap().insert(
            token,
            Session {
                ctx: ctx.clone(),
                expires_at: Instant::now() + Duration::from_secs(86400), // 24 hours
            },
        );

        Ok(ctx)
    }

    pub fn verify_token(&self, token: &str) -> Option<RuntimeContext> {
        let mut sessions = self.sessions.lock().unwrap();
        if let Some(session) = sessions.get(token) {
            if Instant::now() > session.expires_at {
                sessions.remove(token);
                return None;
            }
            return Some(session.ctx.clone());
        }
        None
    }

    #[cfg(test)]
    pub fn expire_session(&self, token: &str) {
        let mut sessions = self.sessions.lock().unwrap();
        if let Some(session) = sessions.get_mut(token) {
            session.expires_at = Instant::now() - Duration::from_secs(1);
        }
    }

    fn cleanup_login_attempts(&self) {
        let mut attempts = self.login_attempts.lock().unwrap();
        if attempts.len() >= MAX_LOGIN_ATTEMPTS {
            // Remove expired entries first
            attempts.retain(|_, (_, time)| time.elapsed() < Duration::from_secs(300));

            // Sentinel: If still over limit, prevent DoS by clearing
            if attempts.len() >= MAX_LOGIN_ATTEMPTS {
                attempts.clear();
            }
        }
    }

    fn cleanup_sessions(&self) {
        let mut sessions = self.sessions.lock().unwrap();
        if sessions.len() >= MAX_SESSIONS {
            // Remove expired sessions
            sessions.retain(|_, session| Instant::now() < session.expires_at);

            // Sentinel: If still over limit, remove arbitrary sessions to prevent OOM
            if sessions.len() >= MAX_SESSIONS {
                // Remove ~10% randomly to make space
                let keys_to_remove: Vec<String> = sessions.keys().take(MAX_SESSIONS / 10).cloned().collect();
                for k in keys_to_remove {
                    sessions.remove(&k);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::datastore::MemoryDataStore;
    use serde_json::json;

    #[tokio::test]
    async fn test_login_timing_mitigation_logic() {
        let store = Arc::new(MemoryDataStore::new());
        let auth = AuthEngine::new(store.clone(), None);

        // 1. Test non-existent user
        let result = auth.login("nonexistent", "password").await;
        assert_eq!(result.err().unwrap(), "Invalid username or password");

        // 2. Test existing user with wrong password
        let hashed = hash_password("correct_password");
        store
            .insert(
                "user",
                json!({
                    "username": "existing",
                    "password": hashed,
                    "role": "user"
                }),
            )
            .await
            .unwrap();

        let result = auth.login("existing", "wrong_password").await;
        assert_eq!(result.err().unwrap(), "Invalid username or password");

        // 3. Test existing user with correct password
        let result = auth.login("existing", "correct_password").await;
        assert!(result.is_ok());
    }

    #[test]
    fn test_v4_hashing() {
        let password = "new_secure_password";
        let stored = hash_password(password);
        assert!(stored.starts_with("v4$"));
        // Ensure iteration count is present
        assert!(stored.contains("$600000$"));

        assert!(verify_password(password, &stored));
        assert!(!verify_password("wrong", &stored));
    }

    #[test]
    fn test_v3_backward_compatibility() {
        // Manually create a v3 hash (100k iterations)
        let password = "legacy_password";
        let salt = "somesalt";
        let iterations = 100_000;
        let mut hash = [0u8; 32];
        pbkdf2::<Hmac<Sha256>>(password.as_bytes(), salt.as_bytes(), iterations, &mut hash).expect("HMAC error");
        let hash_hex = hex::encode(hash);
        let v3_hash = format!("v3${}${}", salt, hash_hex);

        assert!(verify_password(password, &v3_hash));
        assert!(!verify_password("wrong", &v3_hash));
    }

    #[tokio::test]
    async fn test_session_expiration() {
        let store = Arc::new(MemoryDataStore::new());
        let auth = AuthEngine::new(store.clone(), None);

        let hashed = hash_password("secret");
        store
            .insert(
                "user",
                json!({
                    "username": "user1",
                    "password": hashed,
                    "role": "user"
                }),
            )
            .await
            .unwrap();

        // 1. Login
        let ctx = auth.login("user1", "secret").await.unwrap();
        let token = ctx.token.unwrap();

        // 2. Verify Valid
        assert!(auth.verify_token(&token).is_some());

        // 3. Expire Session
        auth.expire_session(&token);

        // 4. Verify Invalid
        assert!(auth.verify_token(&token).is_none());
    }

    #[tokio::test]
    async fn test_login_attempts_memory_exhaustion() {
        let store = Arc::new(MemoryDataStore::new());
        let auth = AuthEngine::new(store.clone(), None);

        // Manually fill attempts to limit + 50
        {
            let mut attempts = auth.login_attempts.lock().unwrap();
            for i in 0..(MAX_LOGIN_ATTEMPTS + 50) {
                let username = format!("user_{}", i);
                attempts.insert(username, (1, Instant::now()));
            }
        }

        // Trigger login which should cleanup
        let _ = auth.login("another_user", "wrong_password").await;

        let attempts = auth.login_attempts.lock().unwrap();
        // Should be cleared because all are "recent", so retain keeps them,
        // but then size > MAX so it clears all.
        // Then "another_user" is inserted (failed attempt).
        assert!(attempts.len() <= MAX_LOGIN_ATTEMPTS);
        assert_eq!(attempts.len(), 1);
    }
}
