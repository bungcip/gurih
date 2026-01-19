use gurih_ir::{EntitySchema, FieldType, Schema, TableSchema};
use sqlx::{AnyPool, Row};
use std::sync::Arc;

pub struct SchemaManager {
    pool: AnyPool,
    schema: Arc<Schema>,
    db_kind: String,
}

impl SchemaManager {
    pub fn new(pool: AnyPool, schema: Arc<Schema>, db_kind: String) -> Self {
        Self {
            pool,
            schema,
            db_kind,
        }
    }

    pub async fn migrate(&self) -> Result<(), String> {
        println!("🔄 Checking database metadata...");
        let mode = self.get_or_init_metadata().await?;
        println!("ℹ️ Running in '{}' mode.", mode);

        if mode == "dev" {
            println!("⚠️ Dev mode detected. Resetting tables...");
            self.drop_all_tables().await?;
        }

        println!("🛠 Creating tables...");
        self.create_tables().await?;

        println!("✅ Schema migration complete.");
        Ok(())
    }

    async fn get_or_init_metadata(&self) -> Result<String, String> {
        let db_kind = &self.db_kind;

        // Check if table exists
        let table_exists: bool = if db_kind == "PostgreSQL" {
            sqlx::query_scalar(
                "SELECT EXISTS (SELECT FROM information_schema.tables WHERE table_name = '_gurih_metadata')"
            )
            .fetch_one(&self.pool)
            .await
            .map_err(|e| e.to_string())?
        } else {
            // SQLite
            let count: i64 = sqlx::query_scalar(
                "SELECT count(*) FROM sqlite_master WHERE type='table' AND name='_gurih_metadata'",
            )
            .fetch_one(&self.pool)
            .await
            .map_err(|e| e.to_string())?;
            count > 0
        };

        if !table_exists {
            // Create table
            sqlx::query("CREATE TABLE _gurih_metadata (key TEXT PRIMARY KEY, value TEXT)")
                .execute(&self.pool)
                .await
                .map_err(|e| e.to_string())?;

            // Insert default mode = dev
            sqlx::query("INSERT INTO _gurih_metadata (key, value) VALUES ('mode', 'dev')")
                .execute(&self.pool)
                .await
                .map_err(|e| e.to_string())?;

            return Ok("dev".to_string());
        }

        // Read mode
        let row = sqlx::query("SELECT value FROM _gurih_metadata WHERE key = 'mode'")
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| e.to_string())?;

        if let Some(row) = row {
            let mode: String = row.try_get("value").unwrap_or("dev".to_string());
            Ok(mode)
        } else {
            // Insert default if missing
            sqlx::query("INSERT INTO _gurih_metadata (key, value) VALUES ('mode', 'dev')")
                .execute(&self.pool)
                .await
                .map_err(|e| e.to_string())?;
            Ok("dev".to_string())
        }
    }

    async fn drop_all_tables(&self) -> Result<(), String> {
        let db_kind = &self.db_kind;
        // Collect all table names from schema (Entities + Tables)
        let mut tables_to_drop = Vec::new();

        for name in self.schema.tables.keys() {
            tables_to_drop.push(name.clone());
        }
        for name in self.schema.entities.keys() {
            tables_to_drop.push(name.clone());
        }

        for table in tables_to_drop {
            let sql = if db_kind == "PostgreSQL" {
                format!("DROP TABLE IF EXISTS \"{}\" CASCADE", table)
            } else {
                format!("DROP TABLE IF EXISTS \"{}\"", table)
            };
            sqlx::query(&sql)
                .execute(&self.pool)
                .await
                .map_err(|e| e.to_string())?;
        }

        Ok(())
    }

    async fn create_tables(&self) -> Result<(), String> {
        // 1. Create Explicit Tables
        for table in self.schema.tables.values() {
            self.create_explicit_table(table).await?;
        }

        // 2. Create Entity Tables
        for entity in self.schema.entities.values() {
            if !self.schema.tables.contains_key(&entity.name) {
                self.create_entity_table(entity).await?;
            }
        }

        Ok(())
    }

    async fn create_explicit_table(&self, table: &TableSchema) -> Result<(), String> {
        let mut sql = format!("CREATE TABLE \"{}\" (", table.name);
        let mut defs = vec![];

        for col in &table.columns {
            let mut def = format!("\"{}\" {}", col.name, col.type_name);
            if col.primary {
                def.push_str(" PRIMARY KEY");
            }
            if col.unique {
                def.push_str(" UNIQUE");
            }
            if let Some(val) = col.props.get("not_null")
                && val == "true"
            {
                def.push_str(" NOT NULL");
            }
            if let Some(val) = col.props.get("default") {
                def.push_str(&format!(" DEFAULT {}", val));
            }

            defs.push(def);
        }

        sql.push_str(&defs.join(", "));
        sql.push(')');

        sqlx::query(&sql)
            .execute(&self.pool)
            .await
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    async fn create_entity_table(&self, entity: &EntitySchema) -> Result<(), String> {
        let mut sql = format!("CREATE TABLE \"{}\" (", entity.name);
        let mut defs = vec![];
        let db_kind = &self.db_kind;

        for field in &entity.fields {
            let col_type = match &field.field_type {
                FieldType::String => "TEXT",
                FieldType::Text => "TEXT",
                FieldType::Integer => {
                    if db_kind == "PostgreSQL" {
                        "INT"
                    } else {
                        "INTEGER"
                    }
                }
                FieldType::Float => {
                    if db_kind == "PostgreSQL" {
                        "DOUBLE PRECISION"
                    } else {
                        "REAL"
                    }
                }
                FieldType::Boolean => {
                    if db_kind == "PostgreSQL" {
                        "BOOLEAN"
                    } else {
                        "INTEGER"
                    }
                }
                FieldType::Date => "DATE",
                FieldType::DateTime => {
                    if db_kind == "PostgreSQL" {
                        "TIMESTAMP"
                    } else {
                        "TEXT"
                    }
                }
                FieldType::Relation => "TEXT",
                FieldType::Enum(_) => "TEXT",
            };

            let mut def = format!("\"{}\" {}", field.name, col_type);

            if field.name == "id" {
                def.push_str(" PRIMARY KEY");
            }

            if field.required {
                def.push_str(" NOT NULL");
            }
            if field.unique {
                def.push_str(" UNIQUE");
            }

            defs.push(def);
        }

        // Process Relationships (belongs_to -> foreign key column)
        for rel in &entity.relationships {
            if rel.rel_type == "belongs_to" {
                let col_name = format!("{}_id", rel.name);
                if !entity.fields.iter().any(|f| f.name == col_name) {
                    let def = format!("\"{}\" TEXT", col_name);
                    defs.push(def);
                }
            }
        }

        sql.push_str(&defs.join(", "));
        sql.push(')');

        sqlx::query(&sql)
            .execute(&self.pool)
            .await
            .map_err(|e| e.to_string())?;
        Ok(())
    }
}
