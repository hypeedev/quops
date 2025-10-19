use proc_macro2::TokenStream;
use quote::quote;
use crate::field::{Field, FieldTrait};
use crate::schema::Schema;
use crate::utils::{camel_to_snake_case, has_bytes_field, parse_schema, validate_enum_schema, validate_record_schema};

fn encode_nullable<F>(field: &Field, var: &TokenStream, get_body: F) -> TokenStream
where
    F: Fn(TokenStream) -> TokenStream
{
    if field.nullable() {
        let body = get_body(quote! { value });
        quote! {
            if let Some(value) = #var {
                writer.write(1, 1)?;
                #body
            } else {
                writer.write(0, 1)?;
            }
        }
    } else {
        get_body(quote! { #var })
    }
}

fn generate_encode_field(field: &Field, field_ident: &TokenStream) -> TokenStream {
    let bits = field.bits();
    debug_assert!(bits <= 255, "Field bits must be in the range of u8 (0-255). Found: {}", bits);
    let bits = bits as u8;
    let field_name = field.name();

    match field {
        Field::Int(int_field) => {
            if let (Some(min), Some(max)) = (int_field.min, int_field.max) {
                encode_nullable(field, field_ident, |var| quote! {
                    if !(#min..=#max).contains(&(#var as i32)) {
                        let err = format!("Value for field '{}' is out of bounds: {}. Expected range: [{}, {}]", stringify!(#var), #var, #min, #max);
                        return Err(::quops::EncodeError::OutOfBounds(err));
                    }
                    writer.write((#var as i32 - #min) as u64, #bits)?;
                })
            } else {
                encode_nullable(field, field_ident, |var| quote! {
                    let bits_width = (64 - (#var as u64).leading_zeros()) as u8;
                    writer.write(bits_width as u64, #bits)?;
                    writer.write(#var as u64, bits_width)?;
                })
            }
        },
        Field::Boolean(_) => {
            encode_nullable(field, field_ident, |var| quote! {
                writer.write(#var as u64, 1)?;
            })
        },
        Field::Enum(_) => {
            encode_nullable(field, field_ident, |var| quote! {
                writer.write(::quops::traits::AsU64::as_u64(&#var)?, #bits)?;
            })
        },
        Field::Bytes(bytes_field) => {
            let max_length = bytes_field.max_length.unwrap_or(2u32.saturating_pow(2u32.saturating_pow(bits as u32)));

            encode_nullable(field, field_ident, |var| {
                let check_bounds = if max_length < u32::MAX {
                    quote! {
                        if #var.len() > #max_length as usize {
                            let err = format!("Bytes length exceeds maximum for field: {:?}, got: {}", #field_name, #var.len());
                            return Err(::quops::EncodeError::OutOfBounds(err));
                        }
                    }
                } else {
                    quote! {}
                };

                quote! {
                    #check_bounds
                    buffers.push(&#var);
                    writer.write(#var.len() as u64, #bits)?;
                }
            })
        },
        Field::Record(record_field) => {
            // If the field is nullable, we need to match it by reference to avoid moving the inner value.
            // Could this be done in a more efficient/readable way?
            let field_ident = if field.nullable() {
                quote! { &#field_ident }
            } else {
                quote! { #field_ident }
            };
            encode_nullable(field, &field_ident, |var| {
                let mut res = quote! {};
                for sub_field in &record_field.fields {
                    let sub_field_ident = syn::Ident::new(&camel_to_snake_case(sub_field.name()), proc_macro2::Span::call_site());
                    let sub_field_ident = quote! { #var.#sub_field_ident };
                    res.extend(generate_encode_field(sub_field, &sub_field_ident));
                }
                res
            })
        },
        Field::Array(array_field) => {
            let item_ident = quote! { item };
            let encode_item = generate_encode_field(&array_field.items_field, &item_ident);
            encode_nullable(field, field_ident, |var| {
                let item_ident = if array_field.items_field.is_primitive() {
                    quote! { &#item_ident }
                } else {
                    item_ident.clone()
                };
                quote! {
                    writer.write(#var.len() as u64, #bits)?;
                    for #item_ident in #var.iter() {
                        #encode_item
                    }
                }
            })
        }
    }
}

#[inline]
pub fn encode(input: syn::DeriveInput) -> TokenStream {
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
                        compile_error!("Encode can only be derived for structs with 'record' schema type");
                    }.into();
                }
            };

            if let Err(err) = validate_record_schema(&schema, data_struct) {
                return quote! {
                    compile_error!(concat!("Schema validation error: ", #err));
                }.into();
            }

            let field_write_calls = schema.fields.iter().map(|field| {
                let field_ident = syn::Ident::new(&camel_to_snake_case(field.name()), proc_macro2::Span::call_site());
                let field_ident = quote! { self.#field_ident };
                generate_encode_field(field, &field_ident)
            }).collect::<Vec<_>>();

            let schema_has_bytes_field = has_bytes_field(&schema.fields);
            let (create_buffers, return_bin) = if schema_has_bytes_field {
                (
                    quote! { let mut buffers = Vec::new(); },
                    quote! {
                        Ok({
                            let mut bin = writer.into_bytes();
                            for buf in buffers.iter().rev() {
                                bin.extend_from_slice(buf);
                            }
                            bin
                        })
                    }
                )
            } else {
                (
                    quote! {},
                    quote! {
                        Ok(writer.into_bytes())
                    }
                )
            };

            let schema_bits = schema.bits();

            let field_bits = schema.fields.iter().filter_map(|f| {
                let name = syn::Ident::new(f.name(), proc_macro2::Span::call_site());
                match f {
                    Field::Array(array_field) => {
                        let items_bits = array_field.items_field.as_ref().bits();
                        Some(quote! { #items_bits * self.#name.len() as u32 })
                    },
                    _ => None
                }
            }).collect::<Vec<_>>();
            let bytes_fields_bytes = schema.fields.iter().filter_map(|f| {
                if !matches!(f, Field::Bytes(_)) { return None; }
                let name = syn::Ident::new(f.name(), proc_macro2::Span::call_site());
                Some(quote! { self.#name.len() as u32 })
            }).collect::<Vec<_>>();

            quote! {
                impl ::quops::traits::Encode for #name {
                    #[inline(always)]
                    fn encode(&self) -> Result<Vec<u8>, ::quops::EncodeError> {
                        let total_bytes = ((#schema_bits #(+ #field_bits)*) + 7) / 8 #(+ #bytes_fields_bytes)*;
                        let mut writer = ::quops::BitWriter::with_capacity(total_bytes as usize);
                        #create_buffers
                        #(#field_write_calls)*
                        #return_bin
                    }
                }
            }.into()
        },
        syn::Data::Enum(data_enum) => {
            let schema = match schema {
                Schema::Enum(enum_schema) => enum_schema,
                _ => {
                    return quote! {
                        compile_error!("Encode can only be derived for enums with 'enum' schema type");
                    }.into();
                }
            };

            if let Err(err) = validate_enum_schema(&schema, data_enum) {
                return quote! {
                    compile_error!(concat!("Schema validation error: ", #err));
                }.into();
            }

            let match_arms = data_enum.variants.iter().enumerate().map(|(index, variant)| {
                let variant_name = &variant.ident;
                let index = index as u64;
                quote! {
                    #name::#variant_name => Ok(#index),
                }
            }).collect::<Vec<_>>();

            quote! {
                impl ::quops::traits::AsU64 for #name {
                    #[inline(always)]
                    fn as_u64(&self) -> Result<u64, ::quops::EncodeError> {
                        match self {
                            #(#match_arms)*
                        }
                    }
                }
            }.into()
        },
        _ => {
            quote! {
                compile_error!("Encode can only be derived for structs and enums");
            }.into()
        }
    }
}
