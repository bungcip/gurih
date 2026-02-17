use gurih_runtime::auth::AuthEngine;
use gurih_runtime::datastore::MemoryDataStore;
use std::sync::Arc;

#[tokio::test]
async fn test_rate_limit_bypass_vulnerability() {
    let store = Arc::new(MemoryDataStore::new());
    let auth = AuthEngine::new(store.clone(), None, None);

    // 1. Target user "admin" - Fail 4 times (Limit is 5)
    for _ in 0..4 {
        let _ = auth.login("admin", "wrong").await;
    }

    // 2. Perform Attack: Flood with random users to trigger cleanup
    // MAX_LOGIN_ATTEMPTS is 10,000.
    // We insert 11,000 random users (count=1 each).
    // This triggers eviction.
    // Eviction sorts by count ASC. "admin" has 4. Flood has 1.
    // Flood users should be evicted. "admin" should be kept.
    for i in 0..11000 {
        let user = format!("flood_{}", i);
        let _ = auth.login(&user, "wrong").await;
    }

    // 3. Check if "admin" is still tracked.
    // 5th attempt:
    // If "admin" was kept (count=4), this proceeds, fails verify, increments to 5.
    // Returns "Invalid username or password".
    let res = auth.login("admin", "wrong").await;
    assert_eq!(res.err().unwrap(), "Invalid username or password", "5th attempt should be allowed but fail auth");

    // 6th attempt:
    // If "admin" was kept (count now 5), this should be BLOCKED.
    // If "admin" was evicted (count reset), this would be allowed (count 1 or 2).
    let res = auth.login("admin", "wrong").await;
    assert_eq!(res.err().unwrap(), "Too many failed attempts. Please try again later.", "6th attempt should be blocked by rate limiter");

    println!("SUCCESS: Rate limit for 'admin' persisted despite flood.");
}
