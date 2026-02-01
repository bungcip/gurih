# Sentinel's Journal

## 2024-05-22 - [CRITICAL] Plaintext Password Storage
**Vulnerability:** Passwords were stored in plaintext in the database.
**Learning:** The application was missing basic security measures for sensitive data. DataEngine was blindly inserting values without transformation.
**Prevention:** Always use hashing (bcrypt/argon2/sha256) for passwords. Added automatic hashing (Salted SHA-256) in DataEngine for fields with `FieldType::Password`.

## 2024-05-24 - [HIGH] Indefinite Session Duration
**Vulnerability:** Authentication tokens were stored in memory without an expiration mechanism, allowing stolen tokens to be used indefinitely until server restart.
**Learning:** `HashMap` based session storage is convenient but dangerous if lifecycle management (expiration/cleanup) is omitted.
**Prevention:** Implemented a `Session` struct with an `expires_at` timestamp and enforced checks on every token verification.
