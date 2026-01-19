use gurih_ir::Schema;
use serde_json::{json, Value};

pub struct FormEngine;

impl FormEngine {
    pub fn new() -> Self {
        Self
    }

    pub fn generate_ui_schema(&self, schema: &Schema, form_name: &str) -> Result<Value, String> {
        let form = schema.forms.get(form_name).ok_or("Form not found")?;
        let entity = schema.entities.get(&form.entity).ok_or("Entity not found")?;

        let mut ui_sections = vec![];

        for section in &form.sections {
            let mut ui_fields = vec![];
            for field_name in &section.fields {
                let field_def = entity.fields.iter().find(|f| &f.name == field_name)
                    .ok_or_else(|| format!("Field {} not found in entity {}", field_name, form.entity))?;
                
                ui_fields.push(json!({
                    "name": field_def.name,
                    "label": field_def.name, 
                    "widget": self.map_field_type_to_widget(&field_def.field_type),
                    "required": field_def.required
                }));
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
    }

    pub fn generate_default_form(&self, schema: &Schema, entity_name: &str) -> Result<Value, String> {
        let entity = schema.entities.get(entity_name).ok_or("Entity not found")?;

        let mut ui_fields = vec![];
        for field_def in &entity.fields {
             ui_fields.push(json!({
                "name": field_def.name,
                "label": field_def.name, 
                "widget": self.map_field_type_to_widget(&field_def.field_type),
                "required": field_def.required
            }));
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
            gurih_ir::FieldType::String => "TextInput".to_string(),
            gurih_ir::FieldType::Text => "TextArea".to_string(),
            gurih_ir::FieldType::Integer => "NumberInput".to_string(),
            gurih_ir::FieldType::Float => "NumberInput".to_string(),
            gurih_ir::FieldType::Boolean => "Checkbox".to_string(),
            gurih_ir::FieldType::Date => "DatePicker".to_string(),
            gurih_ir::FieldType::DateTime => "DateTimePicker".to_string(),
            gurih_ir::FieldType::Enum(_) => "Select".to_string(),
            gurih_ir::FieldType::Relation => "RelationPicker".to_string(),
        }
    }
}
