use std::fmt::{Debug, Display, Formatter};
use std::ops::RangeInclusive;
use darling::FromDeriveInput;
use quote::ToTokens;
use syn::Type;
use crate::field::{Field, FieldTrait};
use crate::schema;
use crate::schema::Schema;

#[derive(Debug, FromDeriveInput)]
#[darling(attributes(schema))]
pub struct SchemaAttr {
    pub path: String,
}

#[derive(Debug)]
pub struct TypeHelper<'a> {
    ty: &'a Type,
}

pub fn snake_to_camel_case(s: &str) -> String {
    let mut result = String::new();
    let mut next_char_uppercase = false;

    for char in s.chars() {
        if char == '_' {
            next_char_uppercase = true;
        } else if next_char_uppercase {
            result.push(char.to_ascii_uppercase());
            next_char_uppercase = false;
        } else {
            result.push(char);
        }
    }

    result
}

pub fn camel_to_snake_case(s: &str) -> String {
    let mut result = String::new();

    for char in s.chars() {
        if char.is_uppercase() {
            if !result.is_empty() {
                result.push('_');
            }
            result.push(char.to_ascii_lowercase());
        } else {
            result.push(char);
        }
    }

    result
}

pub fn valid_types_for_range(range: &RangeInclusive<i128>, field_name: &str) -> Result<Vec<&'static str>, String> {
    let (&min, &max) = (range.start(), range.end());

    if min >= 0 && max <= u8::MAX as i128 {
        Ok(vec!["u8", "u16", "u32", "u64", "u128", "i16", "i32", "i64", "i128"])
    } else if min >= i8::MIN as i128 && max <= i8::MAX as i128 {
        Ok(vec!["i8", "i16", "i32", "i64", "i128"])
    } else if min >= 0 && max <= u16::MAX as i128 {
        Ok(vec!["u16", "u32", "u64", "u128", "i32", "i64", "i128"])
    } else if min >= i16::MIN as i128 && max <= i16::MAX as i128 {
        Ok(vec!["i16", "i32", "i64", "i128"])
    } else if min >= 0 && max <= u32::MAX as i128 {
        Ok(vec!["u32", "u64", "u128", "i64", "i128"])
    } else if min >= i32::MIN as i128 && max <= i32::MAX as i128 {
        Ok(vec!["i32", "i64", "i128"])
    } else if min >= 0 && max <= u64::MAX as i128 {
        Ok(vec!["u64", "u128", "i128"])
    } else if min >= i64::MIN as i128 && max <= i64::MAX as i128 {
        Ok(vec!["i64", "i128"])
    } else {
        Err(format!("Field '{}' has an unsupported range", field_name))
    }
}

impl<'a> TypeHelper<'a> {
    pub fn new(ty: &'a Type) -> Self {
        TypeHelper { ty }
    }

    pub fn get_type(&self) -> Option<String> {
        if let Type::Path(type_path) = &self.ty {
            let segments = &type_path.path.segments;
            if segments.len() == 1 {
                return Some(segments[0].ident.to_string());
            }
        }
        None
    }

    pub fn inner_type(&self) -> Option<TypeHelper<'a>> {
        if let Type::Path(type_path) = &self.ty {
            let segment = &type_path.path.segments[0];
            if let syn::PathArguments::AngleBracketed(args) = &segment.arguments {
                if let Some(inner_type) = args.args.first().and_then(|a| match a {
                    syn::GenericArgument::Type(t) => Some(t),
                    _ => None,
                }) {
                    return Some(TypeHelper::new(inner_type));
                }
            }
        }
        None
    }

    pub fn full_type(&self) -> String {
        if let Type::Path(type_path) = &self.ty {
            let segment = &type_path.path.segments[0];
            if let syn::PathArguments::AngleBracketed(args) = &segment.arguments {
                if let Some(inner_type) = args.args.first().and_then(|a| match a {
                    syn::GenericArgument::Type(t) => Some(t),
                    _ => None,
                }) {
                    let inner_type_str = TypeHelper::new(inner_type).get_type().unwrap_or_default();
                    return format!("{}<{}>", segment.ident, inner_type_str);
                }
            }
            segment.ident.to_string()
        } else {
            self.ty.to_token_stream().to_string()
        }
    }
}

pub fn has_bytes_field(fields: &Vec<Field>) -> bool {
    fields.iter().any(|f| {
        match f {
            Field::Bytes(_) => true,
            Field::Array(array_field) => has_bytes_field(&vec![*array_field.items_field.clone()]),
            Field::Record(record_field) => has_bytes_field(&record_field.fields),
            _ => false,
        }
    })
}

pub fn validate_field_type(field: &Field, type_helper: &TypeHelper) -> Result<(), String> {
    let full_type = type_helper.full_type();
    match field {
        Field::Int(int_field) => {
            let min = int_field.min.unwrap_or(i32::MIN) as i128;
            let max = int_field.max.unwrap_or(i32::MAX) as i128;
            let range = min..=max;
            let mut valid_types = valid_types_for_range(&range, field.name())?
                .iter()
                .map(|s| s.to_string())
                .collect::<Vec<_>>();

            if field.nullable() {
                valid_types = valid_types.iter()
                    .map(|s| format!("Option<{}>", s))
                    .collect();
            }

            if !valid_types.contains(&full_type) {
                return Err(format!("Field '{}' has range {:?}{}. Valid types are: {:?}", field.name(), range, if field.nullable() { " and is nullable" } else { "" }, valid_types))
            }
        },
        Field::Boolean(_) => {
            if type_helper.get_type() != Some("bool".to_string()) && !type_helper.get_type().map_or(false, |t| t.starts_with("Option<")) {
                return Err(format!("Field '{}' is a boolean but has type '{}'", field.name(), full_type))
            }
        },
        Field::Bytes(_) => {
            let expected_type = if field.nullable() { "Option<Vec<u8>>" } else { "Vec<u8>" };
            if full_type != expected_type {
                return Err(format!("Field '{}' should be of type '{}' but has type '{}'", field.name(), expected_type, full_type))
            }
        }
        Field::Array(array_field) => {
            let inner_type_helper = type_helper.inner_type().ok_or(format!("Field '{}' is an array but does not have an inner type", field.name()))?;
            return validate_field_type(&array_field.items_field, &inner_type_helper);
        },
        _ => {}
    }
    Ok(())
}

pub fn validate_record_schema(schema: &schema::RecordSchema, data_struct: &syn::DataStruct) -> Result<(), String> {
    let struct_fields = data_struct.fields.iter().map(|field| {
        let field_name = field.ident.clone().unwrap().to_string();
        let field_name_json = snake_to_camel_case(&field_name);
        let type_helper = TypeHelper::new(&field.ty);
        (field_name, field_name_json, type_helper)
    }).collect::<Vec<_>>();

    for field in &schema.fields {
        let field_name = field.name();
        if !struct_fields.iter().any(|(_, field_name_json, _)| field_name_json == &field_name) {
            return Err(format!("Field '{}' is not present in struct", field_name));
        }
    }

    for (field_name, field_name_json, type_helper) in struct_fields {
        let field = match schema.fields.iter().find(|f| f.name() == field_name_json) {
            Some(f) => f,
            None => return Err(format!("Field '{}' not found in schema", field_name_json)),
        };

        if let Err(err) = validate_field_type(field, &type_helper) {
            return Err(format!("Field '{}' has invalid type: {}", field_name, err));
        }
    }

    Ok(())
}

pub fn validate_enum_schema(schema: &schema::EnumSchema, data_enum: &syn::DataEnum) -> Result<(), String> {
    for variant in &data_enum.variants {
        let variant_name = variant.ident.to_string();
        if !schema.variants.iter().any(|v| *v == variant_name) {
            return Err(format!("Variant '{}' is not present in schema", variant_name));
        }
    }

    for variant in &schema.variants {
        if !data_enum.variants.iter().any(|v| v.ident.to_string() == *variant) {
            return Err(format!("Variant '{}' is not present in enum", variant));
        }
    }

    Ok(())
}

#[derive(Debug)]
pub enum SchemaParseError {
    NoAttribute(String),
    FileNotFound(String),
    ParseError(String),
}

impl Display for SchemaParseError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            SchemaParseError::NoAttribute(msg) => write!(f, "Failed to parse #[schema(...)] attribute: {}", msg),
            SchemaParseError::FileNotFound(path) => write!(f, "Schema file not found: {}", path),
            SchemaParseError::ParseError(msg) => write!(f, "Failed to parse schema: {}", msg),
        }
    }
}

impl std::error::Error for SchemaParseError {}

pub fn parse_schema(input: &syn::DeriveInput) -> Result<Schema, SchemaParseError> {
    let schema_attr = match SchemaAttr::from_derive_input(input) {
        Ok(attr) => attr,
        Err(err) => {
            let err = err.to_string();
            return Err(SchemaParseError::NoAttribute(err));
        }
    };

    let path = std::path::Path::new(&schema_attr.path);

    if !path.exists() {
        let path = path.to_str().unwrap();
        return Err(SchemaParseError::FileNotFound(path.to_string()));
    }

    Schema::parse_from_file(path.to_path_buf())
        .map_err(|err| SchemaParseError::ParseError(err.to_string()))
}