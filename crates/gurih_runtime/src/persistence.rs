use sqlx::{Pool, Postgres, Row, query};
use gurih_ir::{Schema, EntitySchema, TableSchema, FieldType};
use std::sync::Arc;
use std::collections::HashSet;

pub struct SchemaManager {
    pool: Pool<Postgres>,
    schema: Arc<Schema>,
}

impl SchemaManager {
    pub fn new(pool: Pool<Postgres>, schema: Arc<Schema>) -> Self {
        Self { pool, schema }
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
        // Check if table exists
        let table_exists: bool = sqlx::query_scalar(
            "SELECT EXISTS (SELECT FROM information_schema.tables WHERE table_name = '_gurih_metadata')"
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|e| e.to_string())?;

        if !table_exists {
            // Create table
            sqlx::query(
                "CREATE TABLE _gurih_metadata (key TEXT PRIMARY KEY, value TEXT)"
            )
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
        // Collect all table names from schema (Entities + Tables)
        let mut tables_to_drop = Vec::new();

        for name in self.schema.tables.keys() {
            tables_to_drop.push(name.clone());
        }
        for name in self.schema.entities.keys() {
            // Default entity table name is snake_case of entity name?
            // Assuming simple mapping for now.
             tables_to_drop.push(name.clone()); // e.g. "Customer" -> "Customer" (quoted)
        }

        // Drop foreign key constraints first? Or use CASCADE.
        for table in tables_to_drop {
            let sql = format!("DROP TABLE IF EXISTS \"{}\" CASCADE", table);
            sqlx::query(&sql).execute(&self.pool).await.map_err(|e| e.to_string())?;
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
            // Check if explicit table already covers this entity?
            // Current assumption: If explicit table exists with same name, it takes precedence?
            // Or Entity creates a table if no explicit table defined.
            // Requirement: "make table based on 'entity' and 'dbtable'"
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
            // Add other props if needed (precision, not null etc)
            // e.g. if prop "not_null" is true
             if let Some(val) = col.props.get("not_null") {
                 if val == "true" { def.push_str(" NOT NULL"); }
             }
             if let Some(val) = col.props.get("default") {
                 def.push_str(&format!(" DEFAULT {}", val));
             }

            defs.push(def);
        }

        sql.push_str(&defs.join(", "));
        sql.push_str(")");

        sqlx::query(&sql).execute(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(())
    }

    async fn create_entity_table(&self, entity: &EntitySchema) -> Result<(), String> {
        let mut sql = format!("CREATE TABLE \"{}\" (", entity.name);
        let mut defs = vec![];

        // Default ID if not present?
        // Check if 'id' field exists.
        // Usually Entity starts with `id` semantic type.

        for field in &entity.fields {
             let mut col_type = match &field.field_type {
                 FieldType::String => "TEXT", // or VARCHAR
                 FieldType::Text => "TEXT",
                 FieldType::Integer => "INT",
                 FieldType::Float => "DOUBLE PRECISION",
                 FieldType::Boolean => "BOOLEAN",
                 FieldType::Date => "DATE",
                 FieldType::DateTime => "TIMESTAMP",
                 FieldType::Relation => "TEXT", // Store ID as text/uuid
                 FieldType::Enum(_) => "TEXT",
             };

             // Handle "Serial" -> usually implies a sequence or just text that is generated?
             // If field name is "id" and type is integer, make it SERIAL?
             // But Gurih usually uses UUID or Text IDs.
             // If `id` type is Integer, and `serial` prop set?

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
                // e.g. belongs_to "Customer" -> customer_id
                // But if the field is already defined in fields (e.g. implicit?), we shouldn't duplicate.
                // In DSL: belongs_to "Customer" creates a virtual link.
                // Does it create a column?
                // "Creates a 'customer' field in logic, maps to 'customer_id' in DB."

                // We need to check if we should add `customer_id` column.
                // Or maybe the DSL compiler added a field for it?
                // Checking `compiler.rs`: It does NOT add extra fields for relationships.

                // So we need to add the FK column here.
                // Naming convention: {name}_id?
                // name="customer" -> "customer_id"

                let col_name = format!("{}_id", rel.name);
                // Check if this column already exists in fields (user might have defined it explicitly?)
                if !entity.fields.iter().any(|f| f.name == col_name) {
                     // Add it
                     let def = format!("\"{}\" TEXT", col_name); // Assuming TEXT ID
                     defs.push(def);
                }
            }
        }

        sql.push_str(&defs.join(", "));
        sql.push_str(")");

        sqlx::query(&sql).execute(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(())
    }
}
