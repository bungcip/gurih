use sqlx::{PgPool, SqlitePool};

pub mod postgres;
pub mod sqlite;

pub use crate::datastore::DataStore;

#[derive(Clone)]
pub enum DbPool {
    Sqlite(SqlitePool),
    Postgres(PgPool),
}

pub fn validate_identifier(s: &str) -> Result<(), String> {
    if s.is_empty() {
        return Err("Identifier cannot be empty".to_string());
    }
    for c in s.chars() {
        if !c.is_alphanumeric() && c != '_' && c != '-' {
            return Err(format!(
                "Invalid identifier '{}': only alphanumeric, underscore, and dash allowed",
                s
            ));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_identifier() {
        assert!(validate_identifier("valid_name").is_ok());
        assert!(validate_identifier("validName123").is_ok());
        assert!(validate_identifier("valid-name").is_ok());
        assert!(validate_identifier("").is_err());
        assert!(validate_identifier("invalid name").is_err());
        assert!(validate_identifier("invalid'name").is_err());
        assert!(validate_identifier("invalid\"name").is_err());
        assert!(validate_identifier("invalid;").is_err());
        assert!(validate_identifier("drop table users").is_err());
    }
}
