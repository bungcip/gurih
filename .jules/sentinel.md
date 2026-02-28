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

## 2024-05-28 - [HIGH] Hardcoded RBAC Logic
**Vulnerability:** Role-based access control was hardcoded to only recognize "admin" and "HRManager" roles, ignoring any roles and permissions defined in the DSL schema. This creates a security gap where defined policies are not enforced.
**Learning:** Hardcoding security logic decouples it from the configuration/policy definition (DSL), leading to a false sense of security where users believe their policy changes are active.
**Prevention:** Integrated `AuthEngine` with the compiled `Schema` to dynamically load and enforce permissions defined in the DSL `role` blocks.

## 2024-05-29 - [CRITICAL] SQL Injection via Schema Definition (Self-SQLi)
**Vulnerability:** The `QueryEngine` trusted entity names and field identifiers from the schema to be safe, directly interpolating them into SQL queries. A malicious schema (e.g., entity name `User; DROP TABLE users; --`) could execute arbitrary SQL.
**Learning:** In DSL/Schema-driven architectures, the schema definition itself must be treated as untrusted input if it can be influenced by users or if the system aims to be robust against configuration errors. "Internal" names are not always safe.
**Prevention:** Implemented mandatory identifier validation (`validate_identifier`) in `QueryEngine` for table names and `group_by` fields before SQL construction, rejecting any non-alphanumeric identifiers.

## 2024-05-30 - [HIGH] Stack Overflow in Expression Evaluator (DoS)
**Vulnerability:** The recursive expression evaluator lacked a depth limit, allowing a maliciously crafted, deeply nested expression (e.g., `1+(1+(1+...))`) to crash the server with a stack overflow.
**Learning:** Recursive algorithms on user-controlled input must always have explicit depth limits, even in memory-safe languages like Rust, as stack space is finite.
**Prevention:** Implemented a strict `MAX_RECURSION_DEPTH` (250) check in `evaluate` and `needs_async` functions, returning a controlled error instead of crashing.

## 2024-05-31 - [HIGH] Plaintext Passwords in Data Seeder
**Vulnerability:** The CLI data seeder (`FakerEngine`) bypassed authentication logic and inserted default passwords into the database in plaintext ("password123"). While `DataEngine` properly handled hashing for standard user creation, the seeder created insecure records that could be used or accidentally exposed in staging/demo environments.
**Learning:** Security controls must be applied consistently across all data entry points, including development or administrative tools like data seeders or CLI utilities. Bypassing application-layer security (like `DataEngine`) means you must manually replicate its security measures.
**Prevention:** Imported and applied the `hash_password` function directly in `FakerEngine` when generating fields of `FieldType::Password` to ensure seeded data maintains the same security posture as application-generated data.
