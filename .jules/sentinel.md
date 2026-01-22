# Sentinel's Journal

## 2024-05-22 - [CRITICAL] Plaintext Password Storage
**Vulnerability:** Passwords were stored in plaintext in the database.
**Learning:** The application was missing basic security measures for sensitive data. DataEngine was blindly inserting values without transformation.
**Prevention:** Always use hashing (bcrypt/argon2/sha256) for passwords. Added automatic hashing (Salted SHA-256) in DataEngine for fields with `FieldType::Password`.
