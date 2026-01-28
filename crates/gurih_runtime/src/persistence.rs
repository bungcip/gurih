use crate::store::DbPool;
use gurih_ir::{ColumnType, DatabaseType, EntitySchema, FieldType, Schema, Symbol, TableSchema};
use sha2::{Digest, Sha256};
use sqlx::Row;
use std::collections::HashMap;
use std::sync::Arc;

pub struct SchemaManager {
    pool: DbPool,
    schema: Arc<Schema>,
    db_kind: DatabaseType,
}

impl SchemaManager {
    pub fn new(pool: DbPool, schema: Arc<Schema>, db_kind: DatabaseType) -> Self {
        Self { pool, schema, db_kind }
    }

    pub async fn migrate(&self) -> Result<(), String> {
        println!("🔄 Checking database metadata...");
        let mode = self.get_or_init_metadata().await?;
        println!("ℹ️ Running in '{}' mode.", mode);

        if mode == "dev" {
            let current_hash = self.calculate_schema_hash();
            // This is safe because get_or_init_metadata ensures the table exists
            let stored_hash = self.get_stored_schema_hash().await?;

            if stored_hash != Some(current_hash.clone()) {
                println!("⚠️ Schema changed or not initialized. Resetting tables...");
                self.drop_all_tables().await?;

                println!("🛠 Creating tables...");
                self.create_tables().await?;

                self.update_schema_hash(&current_hash).await?;
            } else {
                println!("✅ Schema matches. Skipping table reset.");
            }
        } else {
            println!("🛠 Creating tables if missing...");
            self.create_tables().await?;
        }

        // Always ensure audit table exists
        self.create_audit_table().await?;

        self.apply_seeds().await?;

        println!("✅ Schema migration complete.");
        Ok(())
    }

    async fn apply_seeds(&self) -> Result<(), String> {
        println!("🌱 Applying seeds...");
        for entity in self.schema.entities.values() {
            if let Some(seeds) = &entity.seeds {
                for seed in seeds {
                    self.insert_seed(entity, seed).await?;
                }
            }
        }
        Ok(())
    }

    async fn insert_seed(&self, entity: &EntitySchema, seed: &HashMap<String, String>) -> Result<(), String> {
        let mut cols = vec![];
        let mut placeholders = vec![];
        let mut values_str = vec![]; // Keep values to bind later

        // Sort keys to ensure order? Not strictly necessary but good for debug
        let mut sorted_seed: Vec<_> = seed.iter().collect();
        sorted_seed.sort_by_key(|(k, _)| *k);

        for (k, v) in sorted_seed {
            cols.push(format!("\"{}\"", k));
            values_str.push((k, v));
        }

        for i in 1..=cols.len() {
            if self.db_kind == DatabaseType::Postgres {
                placeholders.push(format!("${}", i));
            } else {
                placeholders.push("?".to_string());
            }
        }

        let sql = format!(
            "INSERT INTO \"{}\" ({}) VALUES ({})",
            entity.name,
            cols.join(", "),
            placeholders.join(", ")
        );

        // Helper to find field type
        let get_type = |name: &str| -> Option<&FieldType> {
            entity
                .fields
                .iter()
                .find(|f| f.name == Symbol::from(name))
                .map(|f| &f.field_type)
        };

        match &self.pool {
            DbPool::Sqlite(p) => {
                let mut query = sqlx::query::<sqlx::Sqlite>(&sql);
                for (k, v) in &values_str {
                    // For SQLite, we can mostly just bind strings, or cast best effort
                    // But boolean needs integer mapping if stored as int
                    let ftype = get_type(k);
                    match ftype {
                        Some(FieldType::Boolean) => {
                            let b = v.parse::<bool>().unwrap_or(false);
                            query = query.bind(b);
                        }
                        Some(FieldType::Integer) => {
                            if let Ok(i) = v.parse::<i64>() {
                                query = query.bind(i);
                            } else {
                                query = query.bind(v.to_string());
                            }
                        }
                        _ => {
                            query = query.bind(v.to_string());
                        }
                    }
                }
                match query.execute(p).await {
                    Ok(_) => Ok(()),
                    Err(e) => {
                        let msg = e.to_string();
                        if msg.contains("UNIQUE constraint failed") || msg.contains("constraint failed") {
                            Ok(())
                        } else {
                            println!("⚠️ Failed to seed {}: {}", entity.name, msg);
                            Ok(())
                        }
                    }
                }
            }
            DbPool::Postgres(p) => {
                let mut query = sqlx::query::<sqlx::Postgres>(&sql);
                for (k, v) in &values_str {
                    let ftype = get_type(k);
                    match ftype {
                        Some(FieldType::Integer) => {
                            let i = v.parse::<i32>().unwrap_or(0);
                            query = query.bind(i);
                        }
                        Some(FieldType::Boolean) => {
                            let b = v.parse::<bool>().unwrap_or(false);
                            query = query.bind(b);
                        }
                        Some(FieldType::Float) => {
                            let f = v.parse::<f64>().unwrap_or(0.0);
                            query = query.bind(f);
                        }
                        _ => {
                            query = query.bind(v.to_string());
                        }
                    }
                }
                match query.execute(p).await {
                    Ok(_) => Ok(()),
                    Err(e) => {
                        let msg = e.to_string();
                        if msg.contains("duplicate key value") {
                            Ok(())
                        } else {
                            println!("⚠️ Failed to seed {}: {}", entity.name, msg);
                            Ok(())
                        }
                    }
                }
            }
        }
    }

    fn calculate_schema_hash(&self) -> String {
        // Collect entities and tables into sorted Vec to ensure deterministic order
        let mut entities: Vec<_> = self.schema.entities.iter().collect();
        entities.sort_by_key(|(k, _)| *k);

        let mut tables: Vec<_> = self.schema.tables.iter().collect();
        tables.sort_by_key(|(k, _)| *k);

        // We only care about entities and tables for DB schema hash
        // We create a temporary structure to hash
        let data = (entities, tables);
        let json = serde_json::to_string(&data).expect("Failed to serialize schema");

        let mut hasher = Sha256::new();
        hasher.update(json);
        hex::encode(hasher.finalize())
    }

    async fn get_stored_schema_hash(&self) -> Result<Option<String>, String> {
        let sql = "SELECT value FROM _gurih_metadata WHERE key = 'schema_hash'";
        let row = match &self.pool {
            DbPool::Sqlite(p) => sqlx::query::<sqlx::Sqlite>(sql)
                .fetch_optional(p)
                .await
                .map_err(|e: sqlx::Error| e.to_string())?
                .map(|r| r.try_get::<String, _>("value").unwrap_or_default()),
            DbPool::Postgres(p) => sqlx::query::<sqlx::Postgres>(sql)
                .fetch_optional(p)
                .await
                .map_err(|e: sqlx::Error| e.to_string())?
                .map(|r| r.try_get::<String, _>("value").unwrap_or_default()),
        };

        if let Some(hash) = row {
            if hash.is_empty() { Ok(None) } else { Ok(Some(hash)) }
        } else {
            Ok(None)
        }
    }

    async fn update_schema_hash(&self, hash: &str) -> Result<(), String> {
        let db_kind = &self.db_kind;
        let sql = if *db_kind == DatabaseType::Postgres {
            "INSERT INTO _gurih_metadata (key, value) VALUES ('schema_hash', $1) ON CONFLICT (key) DO UPDATE SET value = $1"
        } else {
            "INSERT OR REPLACE INTO _gurih_metadata (key, value) VALUES ('schema_hash', ?)"
        };

        match &self.pool {
            DbPool::Sqlite(p) => {
                sqlx::query::<sqlx::Sqlite>(sql)
                    .bind(hash)
                    .execute(p)
                    .await
                    .map_err(|e: sqlx::Error| e.to_string())?;
            }
            DbPool::Postgres(p) => {
                sqlx::query::<sqlx::Postgres>(sql)
                    .bind(hash)
                    .execute(p)
                    .await
                    .map_err(|e: sqlx::Error| e.to_string())?;
            }
        }
        Ok(())
    }

    async fn get_or_init_metadata(&self) -> Result<String, String> {
        let db_kind = &self.db_kind;

        // Check if table exists
        let table_exists: bool = if *db_kind == DatabaseType::Postgres {
            match &self.pool {
                DbPool::Postgres(p) => sqlx::query_scalar(
                    "SELECT EXISTS (SELECT FROM information_schema.tables WHERE table_name = '_gurih_metadata')",
                )
                .fetch_one(p)
                .await
                .map_err(|e| e.to_string())?,
                _ => false,
            }
        } else {
            // SQLite
            match &self.pool {
                DbPool::Sqlite(p) => {
                    let count: i64 = sqlx::query_scalar(
                        "SELECT count(*) FROM sqlite_master WHERE type='table' AND name='_gurih_metadata'",
                    )
                    .fetch_one(p)
                    .await
                    .map_err(|e| e.to_string())?;
                    count > 0
                }
                _ => false,
            }
        };

        if !table_exists {
            let sql_create = "CREATE TABLE _gurih_metadata (key TEXT PRIMARY KEY, value TEXT)";
            let sql_insert = "INSERT INTO _gurih_metadata (key, value) VALUES ('mode', 'dev')";

            match &self.pool {
                DbPool::Sqlite(p) => {
                    sqlx::query::<sqlx::Sqlite>(sql_create)
                        .execute(p)
                        .await
                        .map_err(|e: sqlx::Error| e.to_string())?;
                    sqlx::query::<sqlx::Sqlite>(sql_insert)
                        .execute(p)
                        .await
                        .map_err(|e: sqlx::Error| e.to_string())?;
                }
                DbPool::Postgres(p) => {
                    sqlx::query::<sqlx::Postgres>(sql_create)
                        .execute(p)
                        .await
                        .map_err(|e: sqlx::Error| e.to_string())?;
                    sqlx::query::<sqlx::Postgres>(sql_insert)
                        .execute(p)
                        .await
                        .map_err(|e: sqlx::Error| e.to_string())?;
                }
            }

            return Ok("dev".to_string());
        }

        // Read mode
        let sql = "SELECT value FROM _gurih_metadata WHERE key = 'mode'";
        let row = match &self.pool {
            DbPool::Sqlite(p) => sqlx::query::<sqlx::Sqlite>(sql)
                .fetch_optional(p)
                .await
                .map_err(|e: sqlx::Error| e.to_string())?
                .map(|r| r.try_get::<String, _>("value").unwrap_or("dev".to_string())),
            DbPool::Postgres(p) => sqlx::query::<sqlx::Postgres>(sql)
                .fetch_optional(p)
                .await
                .map_err(|e: sqlx::Error| e.to_string())?
                .map(|r| r.try_get::<String, _>("value").unwrap_or("dev".to_string())),
        };

        if let Some(mode) = row {
            Ok(mode)
        } else {
            // Insert default
            let sql = "INSERT INTO _gurih_metadata (key, value) VALUES ('mode', 'dev')";
            match &self.pool {
                DbPool::Sqlite(p) => {
                    sqlx::query::<sqlx::Sqlite>(sql)
                        .execute(p)
                        .await
                        .map_err(|e: sqlx::Error| e.to_string())?;
                }
                DbPool::Postgres(p) => {
                    sqlx::query::<sqlx::Postgres>(sql)
                        .execute(p)
                        .await
                        .map_err(|e: sqlx::Error| e.to_string())?;
                }
            }
            Ok("dev".to_string())
        }
    }

    async fn drop_all_tables(&self) -> Result<(), String> {
        let db_kind = &self.db_kind;
        let mut tables_to_drop = Vec::new();

        for name in self.schema.tables.keys() {
            tables_to_drop.push(*name);
        }
        for name in self.schema.entities.keys() {
            tables_to_drop.push(*name);
        }

        for table in tables_to_drop {
            let sql = if *db_kind == DatabaseType::Postgres {
                format!("DROP TABLE IF EXISTS \"{}\" CASCADE", table)
            } else {
                format!("DROP TABLE IF EXISTS \"{}\"", table)
            };

            match &self.pool {
                DbPool::Sqlite(p) => {
                    sqlx::query::<sqlx::Sqlite>(&sql)
                        .execute(p)
                        .await
                        .map_err(|e: sqlx::Error| e.to_string())?;
                }
                DbPool::Postgres(p) => {
                    sqlx::query::<sqlx::Postgres>(&sql)
                        .execute(p)
                        .await
                        .map_err(|e: sqlx::Error| e.to_string())?;
                }
            }
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

    async fn create_audit_table(&self) -> Result<(), String> {
        let sql = if self.db_kind == DatabaseType::Postgres {
            r#"CREATE TABLE IF NOT EXISTS "_audit_log" (
                "id" TEXT PRIMARY KEY,
                "entity" TEXT NOT NULL,
                "record_id" TEXT NOT NULL,
                "action" TEXT NOT NULL,
                "user_id" TEXT,
                "diff" TEXT,
                "timestamp" TIMESTAMP DEFAULT CURRENT_TIMESTAMP
            )"#
        } else {
            r#"CREATE TABLE IF NOT EXISTS "_audit_log" (
                "id" TEXT PRIMARY KEY,
                "entity" TEXT NOT NULL,
                "record_id" TEXT NOT NULL,
                "action" TEXT NOT NULL,
                "user_id" TEXT,
                "diff" TEXT,
                "timestamp" TEXT DEFAULT CURRENT_TIMESTAMP
            )"#
        };

        match &self.pool {
            DbPool::Sqlite(p) => {
                sqlx::query::<sqlx::Sqlite>(sql)
                    .execute(p)
                    .await
                    .map_err(|e: sqlx::Error| e.to_string())?;
            }
            DbPool::Postgres(p) => {
                sqlx::query::<sqlx::Postgres>(sql)
                    .execute(p)
                    .await
                    .map_err(|e: sqlx::Error| e.to_string())?;
            }
        }
        Ok(())
    }

    async fn create_explicit_table(&self, table: &TableSchema) -> Result<(), String> {
        let mut sql = format!("CREATE TABLE IF NOT EXISTS \"{}\" (", table.name);
        let mut defs = vec![];

        for col in &table.columns {
            let col_type_str = match &col.type_name {
                ColumnType::Serial => {
                    if self.db_kind == DatabaseType::Postgres {
                        "SERIAL"
                    } else {
                        "INTEGER"
                    }
                }
                ColumnType::Varchar => "VARCHAR",
                ColumnType::Text => "TEXT",
                ColumnType::Integer => {
                    if self.db_kind == DatabaseType::Postgres {
                        "INT"
                    } else {
                        "INTEGER"
                    }
                }
                ColumnType::Float => {
                    if self.db_kind == DatabaseType::Postgres {
                        "DOUBLE PRECISION"
                    } else {
                        "REAL"
                    }
                }
                ColumnType::Boolean => {
                    if self.db_kind == DatabaseType::Postgres {
                        "BOOLEAN"
                    } else {
                        "INTEGER"
                    }
                }
                ColumnType::Date => "DATE",
                ColumnType::Timestamp => {
                    if self.db_kind == DatabaseType::Postgres {
                        "TIMESTAMP"
                    } else {
                        "TEXT"
                    }
                }
                ColumnType::Uuid => {
                    if self.db_kind == DatabaseType::Postgres {
                        "UUID"
                    } else {
                        "TEXT"
                    }
                }
                ColumnType::Json => {
                    if self.db_kind == DatabaseType::Postgres {
                        "JSONB"
                    } else {
                        "TEXT"
                    }
                }
                ColumnType::Custom(s) => s.as_str(),
            };

            let mut def = format!("\"{}\" {}", col.name, col_type_str);
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

        match &self.pool {
            DbPool::Sqlite(p) => {
                sqlx::query::<sqlx::Sqlite>(&sql)
                    .execute(p)
                    .await
                    .map_err(|e: sqlx::Error| e.to_string())?;
            }
            DbPool::Postgres(p) => {
                sqlx::query::<sqlx::Postgres>(&sql)
                    .execute(p)
                    .await
                    .map_err(|e: sqlx::Error| e.to_string())?;
            }
        }
        Ok(())
    }

    async fn create_entity_table(&self, entity: &EntitySchema) -> Result<(), String> {
        let mut sql = format!("CREATE TABLE IF NOT EXISTS \"{}\" (", entity.name);
        let mut defs = vec![];
        let db_kind = &self.db_kind;

        for field in &entity.fields {
            let col_type = match &field.field_type {
                FieldType::Pk
                | FieldType::Serial
                | FieldType::Sku
                | FieldType::Name
                | FieldType::Title
                | FieldType::Description
                | FieldType::Avatar
                | FieldType::Money
                | FieldType::Email
                | FieldType::Phone
                | FieldType::Address
                | FieldType::Password
                | FieldType::String
                | FieldType::Text
                | FieldType::Image
                | FieldType::File
                | FieldType::Relation
                | FieldType::Code
                | FieldType::Custom(_)
                | FieldType::Enum(_) => "TEXT",
                FieldType::Integer => {
                    if *db_kind == DatabaseType::Postgres {
                        "INT"
                    } else {
                        "INTEGER"
                    }
                }
                FieldType::Float => {
                    if *db_kind == DatabaseType::Postgres {
                        "DOUBLE PRECISION"
                    } else {
                        "REAL"
                    }
                }
                FieldType::Boolean => {
                    if *db_kind == DatabaseType::Postgres {
                        "BOOLEAN"
                    } else {
                        "INTEGER"
                    }
                }
                FieldType::Date => "DATE",
                FieldType::Timestamp => {
                    if *db_kind == DatabaseType::Postgres {
                        "TIMESTAMP"
                    } else {
                        "TEXT"
                    }
                }
            };

            let mut def = format!("\"{}\" {}", field.name, col_type);

            if field.name == Symbol::from("id") {
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
            if rel.rel_type == gurih_ir::RelationshipType::BelongsTo {
                let col_name = format!("{}_id", rel.name);
                if !entity.fields.iter().any(|f| f.name == Symbol::from(col_name.as_str())) {
                    let def = format!("\"{}\" TEXT", col_name);
                    defs.push(def);
                }
            }
        }

        sql.push_str(&defs.join(", "));
        sql.push(')');

        match &self.pool {
            DbPool::Sqlite(p) => {
                sqlx::query::<sqlx::Sqlite>(&sql)
                    .execute(p)
                    .await
                    .map_err(|e: sqlx::Error| e.to_string())?;
            }
            DbPool::Postgres(p) => {
                sqlx::query::<sqlx::Postgres>(&sql)
                    .execute(p)
                    .await
                    .map_err(|e: sqlx::Error| e.to_string())?;
            }
        }
        Ok(())
    }
}
