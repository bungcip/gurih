use gurih_ir::utils::to_title_case;
use gurih_ir::{Schema, Symbol};
use serde_json::{Value, json};

pub struct FormEngine;

impl Default for FormEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl FormEngine {
    pub fn new() -> Self {
        Self
    }

    pub fn generate_ui_schema(&self, schema: &Schema, name: Symbol) -> Result<Value, String> {
        // 1. Try direct Form lookup
        let mut target_form = schema.forms.get(&name);

        // 2. Fallback: Search for a form that targets this name as an entity
        if target_form.is_none() {
            target_form = schema.forms.values().find(|f| f.entity == name);
        }

        if let Some(form) = target_form {
            let entity = schema
                .entities
                .get(&form.entity)
                .ok_or_else(|| format!("Entity {} not found", form.entity))?;

            let mut ui_sections = vec![];

            for section in &form.sections {
                let mut ui_fields = vec![];
                for field_name in &section.fields {
                    let ui_field = if let Some(field_def) = entity.fields.iter().find(|f| &f.name == field_name) {
                        let mut field_json = json!({
                            "name": field_def.name,
                            "label": to_title_case(&field_def.name.to_string()),
                            "widget": self.map_field_type_to_widget(&field_def.field_type),
                            "required": field_def.required
                        });

                        if let gurih_ir::FieldType::Enum(variants) = &field_def.field_type {
                            field_json["options"] = json!(
                                variants
                                    .iter()
                                    .map(|v| {
                                        json!({
                                            "label": v,
                                            "value": v
                                        })
                                    })
                                    .collect::<Vec<_>>()
                            );
                        }

                        field_json
                    } else if let Some(rel_def) = entity.relationships.iter().find(|r| &r.name == field_name) {
                        json!({
                            "name": format!("{}_id", rel_def.name.to_string().to_lowercase()),
                            "label": to_title_case(&rel_def.name.to_string()),
                            "widget": "RelationPicker",
                            "required": false // Default for relation
                        })
                    } else {
                        return Err(format!("Field {} not found in entity {}", field_name, form.entity));
                    };

                    ui_fields.push(ui_field);
                }

                ui_sections.push(json!({
                    "title": section.title,
                    "fields": ui_fields
                }));
            }

            Ok(json!({
                "name": form.name,
                "entity": form.entity,
                "layout": ui_sections
            }))
        } else {
            // 3. Fallback: Try generating default form if it's an entity name
            if schema.entities.contains_key(&name) {
                return self.generate_default_form(schema, name);
            }
            Err(format!("Form or Entity '{}' not found", name))
        }
    }

    pub fn generate_default_form(&self, schema: &Schema, entity_name: Symbol) -> Result<Value, String> {
        let entity = schema.entities.get(&entity_name).ok_or("Entity not found")?;

        let mut ui_fields = vec![];

        // Add regular fields
        for field_def in &entity.fields {
            // Skip ID usually or show as readonly? Let's keep it for now but maybe skip 'id' name
            if field_def.name == Symbol::from("id") {
                continue;
            }

            let mut field_json = json!({
                "name": field_def.name,
                "label": to_title_case(field_def.name.as_str()),
                "widget": self.map_field_type_to_widget(&field_def.field_type),
                "required": field_def.required
            });

            if let gurih_ir::FieldType::Enum(variants) = &field_def.field_type {
                field_json["options"] = json!(
                    variants
                        .iter()
                        .map(|v| {
                            json!({
                                "label": v,
                                "value": v
                            })
                        })
                        .collect::<Vec<_>>()
                );
            }

            ui_fields.push(field_json);
        }

        // Add relationship fields
        for rel in &entity.relationships {
            if rel.rel_type == gurih_ir::RelationshipType::BelongsTo {
                ui_fields.push(json!({
                    "name": format!("{}_id", rel.name.to_string().to_lowercase()),
                    "label": to_title_case(&rel.name.to_string()),
                    "widget": "RelationPicker",
                    "required": false
                }));
            }
        }

        let ui_sections = vec![json!({
            "title": "General",
            "fields": ui_fields
        })];

        Ok(json!({
            "name": format!("{}_default", entity_name),
            "entity": entity_name,
            "layout": ui_sections
        }))
    }

    fn map_field_type_to_widget(&self, field_type: &gurih_ir::FieldType) -> String {
        match field_type {
            gurih_ir::FieldType::Pk => "TextInput".to_string(),
            gurih_ir::FieldType::Serial => "TextInput".to_string(),
            gurih_ir::FieldType::Sku => "TextInput".to_string(),
            gurih_ir::FieldType::Name => "TextInput".to_string(),
            gurih_ir::FieldType::Title => "TextInput".to_string(),
            gurih_ir::FieldType::Description => "TextArea".to_string(),
            gurih_ir::FieldType::Avatar => "ImageUpload".to_string(),
            gurih_ir::FieldType::Money => "NumberInput".to_string(),
            gurih_ir::FieldType::Email => "TextInput".to_string(),
            gurih_ir::FieldType::Phone => "TextInput".to_string(),
            gurih_ir::FieldType::Address => "TextArea".to_string(),
            gurih_ir::FieldType::Password => "PasswordInput".to_string(),
            gurih_ir::FieldType::Integer => "NumberInput".to_string(),
            gurih_ir::FieldType::Float => "NumberInput".to_string(),
            gurih_ir::FieldType::Boolean => "Checkbox".to_string(),
            gurih_ir::FieldType::Date => "DatePicker".to_string(),
            gurih_ir::FieldType::Timestamp => "DateTimePicker".to_string(),
            gurih_ir::FieldType::String => "TextInput".to_string(),
            gurih_ir::FieldType::Text => "TextArea".to_string(),
            gurih_ir::FieldType::Image => "ImageUpload".to_string(),
            gurih_ir::FieldType::File => "FileUpload".to_string(),
            gurih_ir::FieldType::Enum(_) => "Select".to_string(),
            gurih_ir::FieldType::Relation => "RelationPicker".to_string(),
            gurih_ir::FieldType::Code => "CodeEditor".to_string(),
            gurih_ir::FieldType::Custom(_) => "TextInput".to_string(),
        }
    }
}

