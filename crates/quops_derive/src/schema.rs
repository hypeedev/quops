use std::collections::HashMap;
use crate::field::{ArrayField, BooleanField, BytesField, EnumField, Field, IntField, FieldTrait, RecordField};

#[derive(Debug)]
pub struct RecordSchema {
    pub fields: Vec<Field>,
    dependencies: HashMap<String, Schema>
}

impl RecordSchema {
    pub fn bits(&self) -> u32 {
        self.fields.iter().map(|f| f.bits()).sum()
    }

    pub fn parse_field(&self, name: &str, value: &serde_json::Value) -> Result<Field, String> {
        if let Some(ty) = value.as_str() {
            match ty {
                "int" => Ok(Field::Int(IntField::new(name, None, None, false)?)),
                "bool" => Ok(Field::Boolean(BooleanField::new(name, false))),
                "bytes" => Ok(Field::Bytes(BytesField::new(name, None, false))),
                "array" => Err(format!("Field '{}' is an array but no schema provided for it", name)),
                _ => {
                    if let Some(dep_schema) = self.dependencies.get(ty) {
                        match dep_schema {
                            Schema::Record(record_schema) => {
                                Ok(Field::Record(RecordField::new(name, record_schema.fields.clone(), false)))
                            },
                            Schema::Enum(enum_schema) => {
                                Ok(Field::Enum(EnumField::new(name, enum_schema.variants.len() as u8, false)))
                            }
                        }
                    } else {
                        Err(format!("Unsupported field type: {}", ty))
                    }
                }
            }
        } else if let Some(map) = value.as_object() {
            let ty = map.get("type").and_then(|v| v.as_str()).expect("Field 'type' is not a string");
            let nullable = map.get("nullable").and_then(|v| v.as_bool()).unwrap_or(false);
            match ty {
                "int" => {
                    let min = map.get("min").and_then(|v| v.as_i64().map(|v| v as i32));
                    let max = map.get("max").and_then(|v| v.as_i64().map(|v| v as i32));

                    if min.is_some() && max.is_some() && min > max {
                        return Err(format!("Invalid range: min = {:?}, max = {:?}", min, max));
                    }

                    Ok(Field::Int(IntField::new(name, min, max, nullable)?))
                },
                "bool" => Ok(Field::Boolean(BooleanField::new(name, nullable))),
                "bytes" => {
                    let max_length = map.get("maxLength")
                        .and_then(|v| v.as_u64())
                        .map(|v| v as u32);
                    Ok(Field::Bytes(BytesField::new(name, max_length, nullable)))
                },
                "array" => {
                    let max_length = map.get("maxLength")
                        .and_then(|v| v.as_u64())
                        .map(|v| v as u32)
                        .unwrap_or_else(|| u32::MAX);
                    let items_type = map.get("items").expect("Array field must have the 'items' field");
                    let items_field = self.parse_field(name, items_type)?;
                    Ok(Field::Array(ArrayField::new(name, max_length, items_field, nullable)))
                }
                _ => {
                    if let Some(dep_schema) = self.dependencies.get(ty) {
                        match dep_schema {
                            Schema::Record(record_schema) => {
                                Ok(Field::Record(RecordField::new(name, record_schema.fields.clone(), nullable)))
                            },
                            Schema::Enum(enum_schema) => {
                                Ok(Field::Enum(EnumField::new(name, enum_schema.variants.len() as u8, nullable)))
                            },
                        }
                    } else {
                        Err(format!("Field '{}' is not a valid type or object", name))
                    }
                }
            }
        } else {
            Err(format!("Field '{}' is not a valid type or object", name))
        }
    }
}

#[derive(Debug)]
pub struct EnumSchema {
    pub variants: Vec<String>,
}

#[derive(Debug)]
pub enum Schema {
    Record(RecordSchema),
    Enum(EnumSchema)
}

impl Schema {
    pub fn parse_from_file(file_path: std::path::PathBuf) -> Result<Self, String> {
        let schema_contents = std::fs::read_to_string(&file_path)
            .expect("Failed to read schema file");
        let schema_value = serde_json::from_str::<serde_json::Value>(&schema_contents)
            .expect("Failed to parse schema file as JSON");

        let ty = schema_value.get("type").and_then(|v| v.as_str())
            .expect("Schema type is not a string");

        match ty {
            "record" => {
                let file_path_parent = file_path.parent().unwrap_or(std::path::Path::new("../../../../../.."));
                let dependencies = schema_value.get("dependencies")
                    .and_then(|v| {
                        let deps = v.as_array().expect("Dependencies are not an array");
                        Some(deps.iter().map(|dep| {
                            let dep_str = dep.as_str().expect("Dependency is not a string");
                            let dep_path = file_path_parent.join(format!("{}.quops", dep_str));
                            let dep_schema = Schema::parse_from_file(dep_path)
                                .expect(&format!("Failed to parse dependency schema: {}", dep_str));
                            (dep_str.to_string(), dep_schema)
                        }).collect::<HashMap<_, _>>())
                    })
                    .unwrap_or(HashMap::new());

                let mut record_schema = RecordSchema {
                    fields: Vec::new(),
                    dependencies
                };

                for (name, field_value) in schema_value.get("fields").and_then(|v| v.as_object()).expect("Fields are not an object") {
                    let field = record_schema.parse_field(name, field_value);
                    match field {
                        Ok(f) => record_schema.fields.push(f),
                        Err(e) => return Err(format!("Failed to parse field '{}': {}", name, e)),
                    }
                }

                Ok(Schema::Record(record_schema))
            }
            "enum" => {
                let variants = schema_value.get("variants")
                    .and_then(|v| v.as_array())
                    .expect("Variants are not an array")
                    .iter()
                    .map(|v| v.as_str().expect("Variant is not a string").to_string())
                    .collect::<Vec<_>>();

                Ok(Schema::Enum(EnumSchema { variants }))
            }
            _ => {
                Err(format!("Unsupported schema type: {}", ty))
            }
        }
    }
}
