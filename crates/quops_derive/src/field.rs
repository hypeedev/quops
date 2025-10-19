pub trait FieldTrait {
    fn bits(&self) -> u32;
    fn name(&self) -> &str;
    fn nullable(&self) -> bool;
    fn is_primitive(&self) -> bool;
}

#[derive(Default, Eq, PartialEq, Clone, Debug, Hash)]
pub struct IntField {
    name: String,
    bits: u8,
    pub min: Option<i32>,
    pub max: Option<i32>,
    nullable: bool,
}

impl IntField {
    pub fn new(name: &str, min: Option<i32>, max: Option<i32>, nullable: bool) -> Result<Self, String> {
        let bits = match (min, max) {
            (Some(min), Some(max)) => {
                if min > max {
                    return Err("Minimum value cannot be greater than maximum value".to_string());
                }
                32 - (max - min + 1).leading_zeros() as u8
            },
            _ => 5,
        } + nullable as u8;
        Ok(IntField {
            name: name.to_string(),
            bits,
            min,
            max,
            nullable,
        })
    }
}

#[derive(Default, Eq, PartialEq, Clone, Debug, Hash)]
pub struct BooleanField {
    name: String,
    bits: u8,
    nullable: bool,
}

impl BooleanField {
    pub fn new(name: &str, nullable: bool) -> Self {
        BooleanField {
            name: name.to_string(),
            bits: 1 + nullable as u8,
            nullable,
        }
    }
}

#[derive(Default, Eq, PartialEq, Clone, Debug, Hash)]
pub struct BytesField {
    name: String,
    bits: u8,
    pub max_length: Option<u32>,
    nullable: bool,
}

impl BytesField {
    pub fn new(name: &str, max_length: Option<u32>, nullable: bool) -> Self {
        let bits = match max_length {
            Some(length) => 32 - length.leading_zeros() as u8,
            None => 5,
        } + nullable as u8;
        BytesField {
            name: name.to_string(),
            bits,
            max_length,
            nullable,
        }
    }
}

#[derive(Default, Eq, PartialEq, Clone, Debug, Hash)]
pub struct EnumField {
    name: String,
    bits: u8,
    pub variants: u8,
    nullable: bool,
}

impl EnumField {
    pub fn new(name: &str, variants: u8, nullable: bool) -> Self {
        EnumField {
            name: name.to_string(),
            bits: 8 - variants.leading_zeros() as u8 + nullable as u8,
            variants,
            nullable,
        }
    }
}

#[derive(Default, Eq, PartialEq, Clone, Debug, Hash)]
pub struct RecordField {
    name: String,
    bits: u32,
    pub fields: Vec<Field>,
    nullable: bool,
}

impl RecordField {
    pub fn new(name: &str, fields: Vec<Field>, nullable: bool) -> Self {
        let bits = fields.iter()
            .map(|f| f.bits())
            .sum::<u32>() + nullable as u32;
        RecordField {
            name: name.to_string(),
            bits,
            fields,
            nullable,
        }
    }
}

#[derive(Eq, PartialEq, Clone, Debug, Hash)]
pub struct ArrayField {
    name: String,
    bits: u8,
    max_length: u32,
    pub items_field: Box<Field>,
    nullable: bool,
}

impl ArrayField {
    pub fn new(name: &str, max_length: u32, field: Field, nullable: bool) -> Self {
        let bits = (32 - max_length.leading_zeros()) as u8 + nullable as u8;
        ArrayField {
            name: name.to_string(),
            bits,
            max_length,
            items_field: Box::new(field),
            nullable,
        }
    }
}

#[derive(Debug, Eq, PartialEq, Clone, Hash)]
pub enum Field {
    Int(IntField),
    Boolean(BooleanField),
    Bytes(BytesField),
    Enum(EnumField),
    Record(RecordField),
    Array(ArrayField),
}

impl FieldTrait for Field {
    fn bits(&self) -> u32 {
        match self {
            Field::Int(field) => field.bits as u32,
            Field::Boolean(field) => field.bits as u32,
            Field::Bytes(field) => field.bits as u32,
            Field::Enum(field) => field.bits as u32,
            Field::Record(field) => field.bits,
            Field::Array(field) => field.bits as u32,
        }
    }

    fn name(&self) -> &str {
        match self {
            Field::Int(field) => &field.name,
            Field::Boolean(field) => &field.name,
            Field::Bytes(field) => &field.name,
            Field::Enum(field) => &field.name,
            Field::Record(field) => &field.name,
            Field::Array(field) => &field.name,
        }
    }

    fn nullable(&self) -> bool {
        match self {
            Field::Int(field) => field.nullable,
            Field::Boolean(field) => field.nullable,
            Field::Bytes(field) => field.nullable,
            Field::Enum(field) => field.nullable,
            Field::Record(field) => field.nullable,
            Field::Array(field) => field.nullable,
        }
    }

    fn is_primitive(&self) -> bool {
        match self {
            Field::Int(_) | Field::Boolean(_) | Field::Bytes(_) | Field::Enum(_) => true,
            Field::Record(_) | Field::Array(_) => false,
        }
    }
}