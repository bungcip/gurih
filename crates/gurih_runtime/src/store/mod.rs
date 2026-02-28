use sqlx::{PgPool, SqlitePool};
use std::path::Path;
use std::sync::Arc;

pub mod memory;
pub mod postgres;
pub mod sqlite;

pub use crate::datastore::DataStore;
use crate::persistence::SchemaManager;
use gurih_ir::{DatabaseType, Schema};
pub use memory::MemoryDataStore;
pub use postgres::PostgresDataStore;
pub use sqlite::SqliteDataStore;
use sqlx::postgres::PgPoolOptions;
use sqlx::sqlite::SqlitePoolOptions;

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

pub async fn init_datastore(schema: Arc<Schema>, base_path: Option<&Path>) -> Result<Arc<dyn DataStore>, String> {
    if let Some(db_config) = &schema.database {
        sqlx::any::install_default_drivers();
        println!("🔌 Connecting to database...");
        // Handle env:DATABASE_URL
        let url = if db_config.url.starts_with("env:") {
            std::env::var(&db_config.url[4..]).unwrap_or_else(|_| "".to_string())
        } else {
            db_config.url.clone()
        };

        if url.is_empty() {
            return Err("Database URL is empty or env var not set.".to_string());
        }

        if db_config.db_type == DatabaseType::Sqlite {
            let mut db_path = url
                .trim_start_matches("sqlite://")
                .trim_start_matches("sqlite:")
                .trim_start_matches("file:")
                .to_string();

            if db_path != ":memory:" {
                let path_obj = Path::new(&db_path);
                let mut full_path = if path_obj.is_relative() {
                    if let Some(parent) = base_path {
                        parent.join(path_obj)
                    } else {
                        std::env::current_dir().map_err(|e| e.to_string())?.join(path_obj)
                    }
                } else {
                    path_obj.to_path_buf()
                };

                // Ensure absolute
                if full_path.is_relative() {
                    full_path = std::env::current_dir().map_err(|e| e.to_string())?.join(full_path);
                }

                // Ensure parent directory exists
                if let Some(parent) = full_path.parent()
                    && !parent.as_os_str().is_empty()
                    && !tokio::fs::try_exists(parent).await.unwrap_or(false)
                {
                    tokio::fs::create_dir_all(parent).await.map_err(|e| e.to_string())?;
                }

                // Explicitly create file if not exists
                if !tokio::fs::try_exists(&full_path).await.unwrap_or(false) {
                    tokio::fs::File::create(&full_path).await.map_err(|e| e.to_string())?;
                }

                db_path = full_path.to_string_lossy().to_string();
            }

            let url = if db_path == ":memory:" {
                "sqlite::memory:".to_string()
            } else {
                format!("sqlite://{}", db_path)
            };

            let mut opts = SqlitePoolOptions::new();
            if url.contains(":memory:") {
                opts = opts.max_connections(1);
            } else {
                opts = opts.max_connections(5);
            }

            let p = opts
                .connect(&url)
                .await
                .map_err(|e| format!("Failed to connect to SQLite DB at {}: {}", url, e))?;

            let pool = DbPool::Sqlite(p.clone());
            let manager = SchemaManager::new(pool, schema.clone(), db_config.db_type.clone());
            manager.migrate().await?;

            Ok(Arc::new(SqliteDataStore::new(p)))
        } else if db_config.db_type == DatabaseType::Postgres {
            let p = PgPoolOptions::new()
                .max_connections(5)
                .connect(&url)
                .await
                .map_err(|e| format!("Failed to connect to Postgres DB: {}", e))?;

            let pool = DbPool::Postgres(p.clone());
            let manager = SchemaManager::new(pool, schema.clone(), db_config.db_type.clone());
            manager.migrate().await?;

            Ok(Arc::new(PostgresDataStore::new(p)))
        } else {
            Err(format!("Unsupported database type: {:?}", db_config.db_type))
        }
    } else {
        println!("⚠️ No database configured. Using in-memory datastore.");
        Ok(Arc::new(MemoryDataStore::new()))
    }
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
