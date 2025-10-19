use std::collections::HashMap;
use proc_macro2::TokenStream;
use quote::quote;
use crate::field::{Field, FieldTrait};
use crate::schema::Schema;
use crate::utils::{camel_to_snake_case, has_bytes_field, parse_schema, snake_to_camel_case, TypeHelper};

fn decode_nullable(field: &Field, body: TokenStream) -> TokenStream {
    if field.nullable() {
        quote! {
            if reader.read(1)? == 1 {
                let value = {
                    #body
                };
                Some(value)
            } else {
                None
            }
        }
    } else {
        quote! {
            #body
        }
    }
}

fn generate_decode_field(field: &Field, field_ident: &syn::Ident, field_type: &str) -> TokenStream {
    let bits = field.bits();
    debug_assert!(bits <= 255, "Field bits must be in the range of u8 (0-255). Found: {}", bits);
    let bits = bits as u8;

    match field {
        Field::Int(int_field) => {
            if let (Some(min), Some(max)) = (int_field.min, int_field.max) {
                decode_nullable(field, quote! {
                    let value = reader.read(#bits)?;
                    let value = (value as i32 + #min) as i32;
                    if !(#min..=#max).contains(&value) {
                        let err = format!("Value for field '{}' is out of bounds: {}. Expected range: [{}, {}]", stringify!(#field_ident), value, #min, #max);
                        return Err(::quops::DecodeError::OutOfBounds(err));
                    }
                    value
                })
            } else {
                decode_nullable(field, quote! {
                    let bits_width = reader.read(#bits)? as u8;
                    reader.read(bits_width)?
                })
            }
        }
        Field::Boolean(_) => {
            decode_nullable(field, quote! {
                reader.read(1)? == 1
            })
        }
        Field::Bytes(_) => {
            decode_nullable(field, quote! {
                let length = reader.read(#bits)? as usize;
                let value = bytes.get(buffers_end_index-length..buffers_end_index);
                buffers_end_index -= length;

                match value {
                    Some(v) => v.to_vec(),
                    None => {
                        let err = format!("Not enough bytes to read field '{}'", stringify!(#field_ident));
                        return Err(::quops::DecodeError::NotEnoughBytes(err))
                    },
                }
            })
        }
        Field::Enum(_) => {
            decode_nullable(field, quote! {
                reader.read(#bits)? as u8
            })
        }
        Field::Record(record_field) => {
            let name = syn::Ident::new(field_type, proc_macro2::Span::call_site());

            let field_names = record_field.fields.iter().map(|field| {
                let field_name = syn::Ident::new(&camel_to_snake_case(field.name()), proc_macro2::Span::call_site());
                let read_call = generate_decode_field(field, &field_name, field_type);
                quote! { #field_name: { #read_call }.try_into()?, }
            }).collect::<Vec<_>>();

            decode_nullable(field, quote! {
                #name {
                    #(#field_names)*
                }
            })
        }
        Field::Array(array_field) => {
            let item_ident = syn::Ident::new("item", proc_macro2::Span::call_site());
            let decode_item = generate_decode_field(&array_field.items_field, &item_ident, field_type);
            decode_nullable(field, quote! {
                let length = reader.read(#bits)? as usize;
                let mut items = Vec::with_capacity(length);
                for _ in 0..length {
                    let #item_ident = {
                        #decode_item
                    };
                    items.push(#item_ident.try_into()?);
                }
                items
            })
        }
    }
}

#[inline]
pub fn decode(input: syn::DeriveInput) -> TokenStream {
    let name = &input.ident;

    let schema = match parse_schema(&input) {
        Ok(schema) => schema,
        Err(err) => {
            let err = err.to_string();
            return quote! {
                compile_error!(concat!("Failed to parse schema: ", #err));
            }.into();
        }
    };

    match &input.data {
        syn::Data::Struct(data_struct) => {
            let schema = match schema {
                Schema::Record(record_schema) => record_schema,
                _ => {
                    return quote! {
                        compile_error!("Decode can only be derived for structs with 'record' schema type");
                    }.into();
                }
            };

            let mut types = HashMap::new();
            for field in &data_struct.fields {
                let field_name = field.ident.as_ref().unwrap().to_string();
                let field_name_json = snake_to_camel_case(&field_name);
                let type_helper = TypeHelper::new(&field.ty);
                let ty = {
                    if let Some(ty) = type_helper.inner_type() {
                        ty.full_type()
                    } else {
                        type_helper.full_type()
                    }
                };
                types.insert(field_name_json, ty);
            }

            let struct_field_names = schema.fields.iter().map(|field| {
                let field_name = field.name();
                let ty = types.get(field_name).expect(&format!("Field '{}' not found in types map", field_name));
                let field_name = syn::Ident::new(&camel_to_snake_case(field.name()), proc_macro2::Span::call_site());
                let read_call = generate_decode_field(field, &field_name, &ty);
                quote! { #field_name: { #read_call }.try_into()?, }
            }).collect::<Vec<_>>();

            let schema_has_bytes_field = has_bytes_field(&schema.fields);
            let create_buffers_end_index = if schema_has_bytes_field {
                quote! { let mut buffers_end_index = bytes.len(); }
            } else {
                quote! {}
            };

            quote! {
                impl ::quops::traits::Decode for #name {
                    #[inline(always)]
                    fn decode(bytes: &[u8]) -> Result<Self, ::quops::DecodeError> {
                        let mut reader = ::quops::BitReader::new(bytes);
                        #create_buffers_end_index

                        Ok(#name {
                            #(#struct_field_names)*
                        })
                    }
                }
            }
        }
        syn::Data::Enum(data_enum) => {
            let schema = match schema {
                Schema::Enum(enum_schema) => enum_schema,
                _ => {
                    return quote! {
                        compile_error!("Decode can only be derived for enums with 'enum' schema type");
                    }.into();
                }
            };

            for variant in &data_enum.variants {
                let variant_name = variant.ident.to_string();
                if !schema.variants.iter().any(|v| *v == variant_name) {
                    return quote! {
                        compile_error!(concat!("Variant '", #variant_name, "' is not present in schema"));
                    }.into();
                }
            }
            for variant in &schema.variants {
                if !data_enum.variants.iter().any(|v| v.ident.to_string() == *variant) {
                    return quote! {
                        compile_error!(concat!("Variant '", #variant, "' is not present in enum"));
                    }.into();
                }
            }

            let match_arms = data_enum.variants.iter().enumerate().map(|(index, variant)| {
                let index = index as u8;
                quote! {
                    #index => Ok(#name::#variant),
                }
            }).collect::<Vec<_>>();

            quote! {
                impl TryInto<#name> for u8 {
                    type Error = ::quops::DecodeError;

                    fn try_into(self) -> Result<#name, Self::Error> {
                        match self {
                            #(#match_arms)*
                            _ => Err(::quops::DecodeError::OutOfBounds(format!("Invalid {} value: {}", stringify!(#name), self))),
                        }
                    }
                }
            }
        }
        _ => quote! {}
    }.into()
}
