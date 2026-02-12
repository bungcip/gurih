# Sentinel's Journal

## 2024-05-22 - [CRITICAL] Plaintext Password Storage
**Vulnerability:** Passwords were stored in plaintext in the database.
**Learning:** The application was missing basic security measures for sensitive data. DataEngine was blindly inserting values without transformation.
**Prevention:** Always use hashing (bcrypt/argon2/sha256) for passwords. Added automatic hashing (Salted SHA-256) in DataEngine for fields with `FieldType::Password`.

## 2024-05-24 - [HIGH] Indefinite Session Duration
**Vulnerability:** Authentication tokens were stored in memory without an expiration mechanism, allowing stolen tokens to be used indefinitely until server restart.
**Learning:** `HashMap` based session storage is convenient but dangerous if lifecycle management (expiration/cleanup) is omitted.
**Prevention:** Implemented a `Session` struct with an `expires_at` timestamp and enforced checks on every token verification.

## 2024-05-25 - [CRITICAL] Unrestricted File Upload (RCE Risk)
**Vulnerability:** The file storage driver accepted any file extension, including `.php`, `.exe`, and `.html`, allowing potential Remote Code Execution or Stored XSS if the storage directory is web-accessible.
**Learning:** `LocalFileDriver` blindly trusted the filename provided by the caller (derived from user input), assuming upstream validation that didn't exist.
**Prevention:** Implemented a mandatory `validate_filename` check in the `FileDriver` trait implementations that enforces a blocklist of dangerous extensions (e.g., `php`, `sh`, `exe`, `svg`) and path traversal checks.

## 2024-05-26 - [HIGH] Timing Attack in Password Verification
**Vulnerability:** `verify_password` returned early if the stored hash format was invalid (e.g., legacy or corrupted), allowing attackers to potentially enumerate users or identify account migration status by measuring response time.
**Learning:** Convenience checks (like checking string prefix/format) can introduce side channels if they bypass expensive operations.
**Prevention:** Always perform the expensive cryptographic operation (PBKDF2/Argon2) regardless of input validity to ensure constant-time execution. Use dummy salts/hashes if necessary.

## 2024-05-27 - [HIGH] Unbounded Memory Growth in AuthEngine
**Vulnerability:** `login_attempts` and `sessions` maps in `AuthEngine` grew indefinitely, allowing Denial of Service via memory exhaustion by spamming login requests with unique usernames.
**Learning:** In-memory state tracking for security (rate limiting, sessions) must always have upper bounds or eviction policies.
**Prevention:** Implemented `cleanup_login_attempts` and `cleanup_sessions` to enforce `MAX_LOGIN_ATTEMPTS` and `MAX_SESSIONS` limits, clearing excessive entries when thresholds are reached.
