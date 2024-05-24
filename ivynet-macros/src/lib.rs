use std::{error::Error, fmt, fmt::Debug};

use proc_macro::TokenStream;
use proc_macro2::{token_stream::TokenStream as TokenStream2, Span};
use quote::quote;
use syn::{parse_macro_input, Field, ItemStruct, Lit::Str};

#[derive(Debug)]
enum HexError {
    InvalidCharacter(char),
    InvalidStringLength(usize),
}

impl Error for HexError {}

impl fmt::Display for HexError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::InvalidCharacter(char) => {
                write!(f, "Invalid character {char}")
            }
            Self::InvalidStringLength(length) => write!(f, "Invalid string length {length}"),
        }
    }
}

fn hex_decode<T: AsRef<[u8]>>(hex: T) -> Result<Vec<u8>, HexError> {
    let mut hex = hex.as_ref();
    let mut length = hex.len();

    if length == 42 && hex[0] == b'0' && (hex[1] == b'x' || hex[1] == b'X') {
        length -= 2;
        hex = &hex[2..];
    }
    if length != 40 {
        return Err(HexError::InvalidStringLength(length));
    }

    let hex_value = |char: u8| -> Result<u8, HexError> {
        match char {
            b'A'..=b'F' => Ok(char - b'A' + 10),
            b'a'..=b'f' => Ok(char - b'a' + 10),
            b'0'..=b'9' => Ok(char - b'0'),
            _ => Err(HexError::InvalidCharacter(char as char)),
        }
    };

    let mut bytes = Vec::with_capacity(length / 2);
    for chunk in hex.chunks(2) {
        let msd = hex_value(chunk[0])?;
        let lsd = hex_value(chunk[1])?;
        bytes.push(msd << 4 | lsd);
    }

    Ok(bytes)
}

#[proc_macro]
pub fn h160(input: TokenStream) -> TokenStream {
    let bytes = hex_decode(input.to_string()).expect("hex string");
    let expanded = quote! {
        H160([#(#bytes,)*])
    };

    expanded.into()
}

#[proc_macro_attribute]
pub fn cql_entity(attr: TokenStream, input: TokenStream) -> TokenStream {
    let attr = parse_macro_input!(attr as syn::AttributeArgs);
    let mut primary_keys = Vec::new();
    let mut table_name = "".to_string();
    for nested_meta in attr {
        if let syn::NestedMeta::Meta(syn::Meta::NameValue(nv)) = nested_meta {
            if nv.path.is_ident("primary_key") {
                if let Str(lit_str) = nv.lit {
                    primary_keys = lit_str
                        .value()
                        .split(',')
                        .map(|s| s.trim().to_string())
                        .collect();
                }
            } else if nv.path.is_ident("table_name") {
                if let Str(lit_str) = nv.lit {
                    table_name = lit_str.value();
                }
            }
        }
    }

    if table_name.is_empty() {
        panic!("Table name not specified");
    }

    let input_clone = input.clone();
    let ast = parse_macro_input!(input as ItemStruct);
    let struct_name = &ast.ident;
    let fields = ast.fields;
    let field_names: Vec<_> = fields.iter().map(|field| &field.ident).collect();

    for key in &primary_keys {
        if !field_names
            .iter()
            .any(|&name| name.clone().unwrap().to_string() == *key)
        {
            panic!("Primary key field '{}' not found in struct.", key);
        }
    }

    let mut index = 0usize;
    let field_parsing = fields.iter().map(|field| {
        let field_name = &field.ident;
        let field_type = &field.ty;
        let current_index = index;
        index += 1;

        let is_enum = field.attrs.iter().any(|attr| {
            attr.path.is_ident("doc") && attr.tokens.to_string().contains("\"is_enum\"")
        });

        if let syn::Type::Path(type_path) = field_type {
            if let Some(ident) = type_path.path.get_ident() {
                let conversion_code =
                    generate_conversion(&ident.to_string(), current_index, is_enum);
                quote! {
                    let #field_name = {
                        #conversion_code
                    };
                }
            } else if type_path.path.segments.len() == 1
                && type_path.path.segments[0].ident == "Option"
            {
                if let syn::PathArguments::AngleBracketed(angle_bracketed_args) =
                    &type_path.path.segments[0].arguments
                {
                    if let Some(syn::GenericArgument::Type(syn::Type::Path(inner_type_path))) =
                        angle_bracketed_args.args.first()
                    {
                        if let Some(inner_ident) = inner_type_path.path.get_ident() {
                            let conversion_code = generate_option_conversion(
                                &inner_ident.to_string(),
                                current_index,
                                is_enum,
                            );
                            quote! {
                                let #field_name = #conversion_code;
                            }
                        } else if inner_type_path.path.segments.len() == 1
                            && inner_type_path.path.segments[0].ident == "DateTime"
                        {
                            let conversion_code =
                                generate_option_conversion("DateTime<Utc>", current_index, false);

                            quote! {
                                let #field_name = #conversion_code;
                            }
                        } else {
                            panic!(
                                "Unsupported inner type for Option {:?}",
                                quote! {#inner_type_path}.to_string()
                            );
                        }
                    } else {
                        panic!("Unsupported generic argument for Option");
                    }
                } else {
                    panic!("Unsupported path arguments for Option");
                }
            } else if type_path.path.segments.len() == 1
                && type_path.path.segments[0].ident == "DateTime"
            {
                let conversion_code = generate_conversion("DateTime<Utc>", current_index, false);
                quote! {
                    let #field_name = {
                        #conversion_code
                    };
                }
            } else {
                let path_display = quote! { #type_path.path }.to_string();

                panic!(
                    "Unsupported type for nested field {:?}: {:?}",
                    field_name, path_display
                );
            }
        } else {
            let type_display = quote! { #field_type }.to_string();

            panic!(
                "Unsupported type for X field {:?}: {:?}",
                field_name, type_display
            );
        }
    });

    let param_pushes: Vec<_> = primary_keys
        .iter()
        .enumerate()
        .map(|(_, key)| {
            let field_ident = syn::Ident::new(key, Span::call_site());
            let field = fields
                .iter()
                .find(|field| field.ident == Some(field_ident.clone()))
                .unwrap();
            let is_enum = field.attrs.iter().any(|attr| {
                attr.path.is_ident("doc") && attr.tokens.to_string().contains("\"is_enum\"")
            });
            let field_parse = parse_field_from_standard_to_cql_bullshit_arg(field, is_enum);
            quote! {
                #key => {
                    params.insert(#key, #field_parse);
                }
            }
        })
        .collect();

    let param_self_pushes: Vec<_> = primary_keys
        .iter()
        .enumerate()
        .map(|(_, key)| {
            let field_ident = syn::Ident::new(key, Span::call_site());
            let field = fields
                .iter()
                .find(|field| field.ident == Some(field_ident.clone()))
                .unwrap();
            let is_enum = field.attrs.iter().any(|attr| {
                attr.path.is_ident("doc") && attr.tokens.to_string().contains("\"is_enum\"")
            });
            let field_parse = parse_field_from_standard_to_cql_bullshit(field, is_enum);
            quote! {
                params.insert(#key, #field_parse);
            }
        })
        .collect();

    let param_list: Vec<_> = primary_keys
        .iter()
        .enumerate()
        .map(|(_, key)| {
            let field_ident = syn::Ident::new(key, Span::call_site());
            let field = fields
                .iter()
                .find(|field| field.ident == Some(field_ident.clone()))
                .unwrap();
            let is_enum = field.attrs.iter().any(|attr| {
                attr.path.is_ident("doc") && attr.tokens.to_string().contains("\"is_enum\"")
            });

            let field_parse = parse_field_from_standard_to_cql_bullshit_arg(field, is_enum);
            quote! {
                params.insert(#key, #field_parse);
            }
        })
        .collect();

    let param_self_list: Vec<_> = fields
        .iter()
        .enumerate()
        .map(|(_, field)| {
            let field_ident = field.clone().ident.unwrap();
            let field = fields
                .iter()
                .find(|field| field.ident == Some(field_ident.clone()))
                .unwrap();
            let field_name = field_ident.to_string();
            let is_enum = field.attrs.iter().any(|attr| {
                attr.path.is_ident("doc") && attr.tokens.to_string().contains("\"is_enum\"")
            });
            let field_parse = parse_field_from_standard_to_cql_bullshit(field, is_enum);
            quote! {
                params.insert(#field_name, #field_parse);
            }
        })
        .collect();

    let all_columns_joined = field_names
        .iter()
        .map(|&name| name.clone().unwrap().to_string())
        .collect::<Vec<_>>()
        .join(", ");

    let where_clauses = primary_keys.iter().map(|key| {
        let where_clause = format!("{key} = :{key}");
        quote! { #where_clause }
    });

    let primary_key_length = primary_keys.len();
    let count_where = primary_keys
        .iter()
        .map(|key| format!("{key} = :{key}"))
        .collect::<Vec<_>>()
        .join(" AND ");

    let upsert_query = format!(
        "INSERT INTO {} ({}) VALUES ({})",
        table_name,
        all_columns_joined,
        all_columns_joined
            .split(", ")
            .map(|s| format!(":{}", s))
            .collect::<Vec<_>>()
            .join(", ")
    );

    let count_query = format!("SELECT COUNT (1) FROM {} WHERE {}", table_name, count_where);

    let select_list_query = format!(
        "SELECT {} FROM {} WHERE {}",
        all_columns_joined, table_name, count_where
    );

    let select_single_query = format!(
        "SELECT {} FROM {} WHERE {} LIMIT 1",
        all_columns_joined, table_name, count_where
    );

    let max_of_query = format!("SELECT MAX(!) FROM {}", table_name);
    let min_of_query = format!("SELECT MIN(!) FROM {}", table_name);

    let select_distinct_query = format!("SELECT DISTINCT ! FROM {}", table_name);

    let select_single_where_max_query = format!(
        "SELECT {} FROM {} WHERE ! = ? LIMIT 1",
        all_columns_joined, table_name
    );

    let select_all_query = format!("SELECT {} FROM {}", all_columns_joined, table_name);

    let primary_keys_len = primary_keys.len();
    let primary_keys_tokens: Vec<_> = primary_keys.iter().map(|k| quote! { #k }).collect();
    let primary_keys_distinct = primary_keys_tokens.clone();

    let cql_queryable_impl = quote! {
            impl common::entity::CqlQueryable for #struct_name {
                fn select_single_query() -> String {
                    #select_single_query.to_string()
                }

                fn select_count_query() -> String  {
                    #count_query.to_string()
                }

                fn select_query() -> String {
                    #select_list_query.to_string()
                }

                fn max_of_queries(column: &str) -> (String, String) {
                    (#max_of_query.replace("!", column), #select_single_where_max_query.replace("!", column))
                }

                fn min_of_query(column: &str) -> String {
                    #min_of_query.replace("!", column)
                }

                fn select_distinct_query(column: &str) -> String {
                    const PRIMARY_KEYS: [&str; #primary_keys_len] = [ #(#primary_keys_distinct),* ];

                    let mut columns = PRIMARY_KEYS
                        .into_iter()
                        .take_while(|&key| key != column)
                        .collect::<Vec<_>>();

                    columns.push(column);

                    let columns_str = columns.join(", ");

                    #select_distinct_query.replace("!", &columns_str)
                }

                fn select_query_with_depth(depth:usize) -> String {
                        if depth > #primary_key_length {
                            panic!("Depth exceeds the number of primary key fields.");
                        }

                        let where_clause = [#(#where_clauses),*][..depth].join(" AND ");
                        let query = format!("SELECT {} FROM {} WHERE {}", #all_columns_joined, #table_name, where_clause);

                        query
                }

                fn select_all_query() -> String
                {
                    #select_all_query.to_string()
                }

                fn upsert_query(
                    &self,
                ) -> (
                    String,
                    std::collections::HashMap<&str, scylla::frame::response::result::CqlValue>,
                ) {
                    let mut params: std::collections::HashMap<&str, scylla::frame::response::result::CqlValue> = std::collections::HashMap::new();
                    #(#param_self_list)*

                    (#upsert_query.to_string(), params)
                }

                fn get_primary_key_parameters(&self,) -> std::collections::HashMap<&str, scylla::frame::response::result::CqlValue> {
                    let mut params: std::collections::HashMap<&str, scylla::frame::response::result::CqlValue> = std::collections::HashMap::new();
                    #(#param_self_pushes)*

                    params
                }
            }

    };

    let field_names_length = field_names.len();
    let mut expanded = TokenStream2::from(input_clone);

    let primary_key_types_vec: Vec<_> = primary_keys
        .iter()
        .map(|key| {
            let field_ident = syn::Ident::new(key, Span::call_site());
            let field = fields
                .iter()
                .find(|field| field.ident == Some(field_ident.clone()))
                .unwrap();

            field
        })
        .filter(|field| {
            let field_name = &field.ident;
            primary_keys.contains(&field_name.clone().unwrap().to_string())
        })
        .map(|field| (&field.ident, &field.ty))
        .collect();

    let fn_args: Vec<_> = primary_key_types_vec
        .iter()
        .map(|(ident, ty)| quote! { #ident: #ty })
        .collect();

    expanded.extend( quote! {
        impl scylla::FromRow for #struct_name {
            fn from_row(row: scylla::frame::response::result::Row) -> Result<Self, scylla::cql_to_rust::FromRowError> {
                const EXPECTED: usize = #field_names_length;
                let actual = row.columns.len();
                let mut columns = row.columns.iter().cloned();
                #(#field_parsing)*
                Ok(Self {
                    #(#field_names),*
                })
            }
        }

        impl #struct_name {
            pub fn get_primary_keys_where_clause(#(#fn_args),*) -> std::collections::HashMap<&'static str, scylla::frame::response::result::CqlValue> {
                let mut params = std::collections::HashMap::new();
                #(#param_list)*
                params
            }

            pub fn get_primary_keys_where_clause_with_depth(#(#fn_args),*, depth: usize) -> std::collections::HashMap<&'static str, scylla::frame::response::result::CqlValue> {
                let mut params = std::collections::HashMap::new();
                const PRIMARY_KEYS: [&str; #primary_keys_len] = [ #(#primary_keys_tokens),* ];
                let mut i = 0;
                for key in PRIMARY_KEYS.iter() {
                    if i >= depth {
                        break;
                    }
                    match *key {
                        #(#param_pushes)*
                        _ => {panic!("Unsupported primary key type");}
                    }
                    i += 1;
                }

                params
            }
        }

        #cql_queryable_impl
    });

    expanded.into()
}
fn generate_cql_value(ident: &str, field_name: &TokenStream2) -> Option<TokenStream2> {
    match ident {
        "H256" | "H160" => Some(quote! {
            scylla::frame::response::result::CqlValue::Varint(common::conversions::address_bytes_to_bigint(#field_name.as_bytes()))
        }),
        "U256" | "I256" => Some(quote! {
            {
                let mut buf = [0u8; 32];
                #field_name.to_big_endian(&mut buf);
                scylla::frame::response::result::CqlValue::Varint(common::conversions::address_bytes_to_bigint(buf.as_slice()))
            }
        }),
        "String" => Some(quote! {
            scylla::frame::response::result::CqlValue::Text(#field_name.clone())
        }),
        "u64" => Some(quote! {
            scylla::frame::response::result::CqlValue::BigInt(#field_name as i64)
        }),
        "i32" => Some(quote! {
            scylla::frame::response::result::CqlValue::Int(#field_name)
        }),
        "u32" => Some(quote! {
            scylla::frame::response::result::CqlValue::Int(#field_name as i32)
        }),
        "f64" => Some(quote! {
            scylla::frame::response::result::CqlValue::Double(#field_name)
        }),
        "bool" => Some(quote! {
            scylla::frame::response::result::CqlValue::Boolean(#field_name)
        }),
        "Uuid" => Some(quote! {
            scylla::frame::response::result::CqlValue::Uuid(#field_name)
        }),
        "Enum" => Some(quote! {
            scylla::frame::response::result::CqlValue::Text(#field_name.to_string())
        }),
        _ => None,
    }
}

fn generate_option_cql_value(inner_ident: &str, field_name: &TokenStream2) -> Option<TokenStream2> {
    match inner_ident {
        "Uuid" | "i32" | "u32" | "H256" | "H160" | "U256" | "I256" | "bool" | "f64" => {
            let conversion = generate_cql_value(inner_ident, &quote! { val })?;
            Some(quote! {
                match #field_name {
                    Some(val) => #conversion,
                    None => scylla::frame::response::result::CqlValue::Empty,
                }
            })
        }
        "String" | "Enum" => {
            let conversion = generate_cql_value(inner_ident, &quote! { val })?;
            Some(quote! {
                match #field_name.clone() {
                    Some(val) => #conversion,
                    None => scylla::frame::response::result::CqlValue::Empty,
                }
            })
        }
        "DateTime" => Some(quote! {
            match #field_name {
                Some(val) => scylla::frame::response::result::CqlValue::BigInt(val.timestamp_millis()),
                None => scylla::frame::response::result::CqlValue::Empty,
            }
        }),
        _ => None,
    }
}

fn parse_field_from_standard_to_cql_bullshit(field: &Field, is_enum: bool) -> TokenStream2 {
    let field_name = field.ident.as_ref().unwrap();
    let self_field_name = quote! { self.#field_name };

    match &field.ty {
        syn::Type::Path(type_path) => {
            if let Some(last_segment) = type_path.path.segments.last() {
                let type_name = if is_enum {
                    "Enum".to_string()
                } else {
                    last_segment.ident.to_string()
                };
                if let Some(result) = generate_cql_value(&type_name, &self_field_name) {
                    return result;
                }

                if last_segment.ident == "Option" {
                    if let syn::PathArguments::AngleBracketed(angle_bracketed_args) =
                        &last_segment.arguments
                    {
                        if let Some(syn::GenericArgument::Type(syn::Type::Path(inner_type))) =
                            angle_bracketed_args.args.first()
                        {
                            let inner_type_ident = &inner_type.path.segments.last().unwrap().ident;
                            if let Some(result) = generate_option_cql_value(
                                &inner_type_ident.to_string(),
                                &self_field_name,
                            ) {
                                return result;
                            }
                        }
                    }
                } else if last_segment.ident == "DateTime" {
                    if let syn::PathArguments::AngleBracketed(angle_bracketed_args) =
                        &last_segment.arguments
                    {
                        if let Some(syn::GenericArgument::Type(syn::Type::Path(inner_type))) =
                            angle_bracketed_args.args.first()
                        {
                            let inner_type_ident = &inner_type.path.segments.last().unwrap().ident;
                            if inner_type_ident == "Utc" {
                                return quote! {
                                        scylla::frame::response::result::CqlValue::BigInt(#self_field_name.timestamp_millis())
                                };
                            }
                        }
                    }
                }

                panic!(
                    "Unsupported type for field {}: {} but is enum: {}",
                    field_name, last_segment.ident, is_enum
                );
            } else {
                panic!("Unsupported type for field {}", field_name);
            }
        }
        _ => {
            panic!("Unsupported type in parse field from standard to cql bullshit");
        }
    }
}

fn parse_field_from_standard_to_cql_bullshit_arg(field: &Field, is_enum: bool) -> TokenStream2 {
    let field_name = &field.ident;
    let field_name = quote! { #field_name };

    match &field.ty {
        syn::Type::Path(type_path) => {
            if let Some(last_segment) = type_path.path.segments.last() {
                let type_name = if is_enum {
                    "Enum".to_string()
                } else {
                    last_segment.ident.to_string()
                };
                if let Some(result) = generate_cql_value(&type_name, &field_name) {
                    return result;
                }

                if last_segment.ident == "Option" {
                    if let syn::PathArguments::AngleBracketed(angle_bracketed_args) =
                        &last_segment.arguments
                    {
                        if let Some(syn::GenericArgument::Type(syn::Type::Path(inner_type))) =
                            angle_bracketed_args.args.first()
                        {
                            let inner_type_ident = &inner_type.path.segments.last().unwrap().ident;
                            if let Some(result) = generate_option_cql_value(
                                &inner_type_ident.to_string(),
                                &field_name,
                            ) {
                                return result;
                            }
                        }
                    }
                } else if last_segment.ident == "DateTime" {
                    if let syn::PathArguments::AngleBracketed(angle_bracketed_args) =
                        &last_segment.arguments
                    {
                        if let Some(syn::GenericArgument::Type(syn::Type::Path(inner_type))) =
                            angle_bracketed_args.args.first()
                        {
                            let inner_type_ident = &inner_type.path.segments.last().unwrap().ident;
                            if inner_type_ident == "Utc" {
                                return quote! {
                                        scylla::frame::response::result::CqlValue::BigInt(#field_name.timestamp_millis())
                                };
                            }
                        }
                    }
                }

                panic!(
                    "Unsupported type for field {}: {} but is enum: {}",
                    field_name, last_segment.ident, is_enum
                );
            } else {
                panic!("Unsupported type for field {}", field_name);
            }
        }
        _ => {
            panic!("Unsupported type in parse field from standard to cql bullshit");
        }
    }
}

fn generate_conversion(field_type: &str, current_index: usize, is_enum: bool) -> TokenStream2 {
    match field_type {
        "H256" => quote! {
            {
                let value: num_bigint::BigInt = common::database::from_cql(columns.next(), #current_index, actual, EXPECTED)?;
                common::conversions::bigint_to_h256(value)
            }
        },
        "U256" => quote! {
            {
                let value: num_bigint::BigInt = common::database::from_cql(columns.next(), #current_index, actual, EXPECTED)?;
                common::conversions::u256_from_bigint(&value)
            }
        },
        "I256" => {
            quote! {
                {
                    let value: num_bigint::BigInt = common::database::from_cql(columns.next(), #current_index, actual, EXPECTED)?;
                    common::conversions::i256_from_bigint(&value)
                }
            }
        }
        "H160" => {
            quote! {
                {
                    let value: num_bigint::BigInt = common::database::from_cql(columns.next(), #current_index, actual, EXPECTED)?;
                    common::conversions::bigint_to_h160(value)
                }
            }
        }
        "DateTime<Utc>" => {
            quote! {
                {
                    let value: i64 = common::database::from_cql(columns.next(), #current_index, actual, EXPECTED)?;
                    common::conversions::milliseconds_to_datetime_utc(value)
                }
            }
        }
        "u32" => {
            quote! {
                {
                    let value: i32 = common::database::from_cql(columns.next(), #current_index, actual, EXPECTED)?;
                    value as u32
                }
            }
        }
        "u64" => {
            quote! {
                {
                    let value: i64 = common::database::from_cql(columns.next(), #current_index, actual, EXPECTED)?;
                    value as u64
                }
            }
        }
        _ => {
            if is_enum {
                let enum_ident: syn::Ident = syn::Ident::new(field_type, Span::call_site());

                quote! {
                    {
                        let value: String = common::database::from_cql(columns.next(), #current_index, actual, EXPECTED)?;
                        value.parse::<#enum_ident>().unwrap()
                    }
                }
            } else {
                quote! {
                    common::database::from_cql(columns.next(), #current_index, actual, EXPECTED)?
                }
            }
        }
    }
}

fn generate_option_conversion(
    inner_type: &str,
    current_index: usize,
    is_enum: bool,
) -> TokenStream2 {
    match inner_type {
        "H256" => quote! {
            {
                let value = common::database::from_cql::<Option<num_bigint::BigInt>>(columns.next(), #current_index, actual, EXPECTED)?;
                match value {
                    Some(value) => Some(common::conversions::bigint_to_h256(value)),
                    None => None
                }
            }
        },
        "U256" => quote! {
            {
                let value = common::database::from_cql::<Option<num_bigint::BigInt>>(columns.next(), #current_index, actual, EXPECTED)?;
                match value {
                    Some(value) => Some(common::conversions::u256_from_bigint(&value)),
                    None => None
                }
            }
        },
        "I256" => {
            quote! {
                {
                    let value = common::database::from_cql::<Option<num_bigint::BigInt>>(columns.next(), #current_index, actual, EXPECTED)?;
                    match value {
                        Some(value) => Some(common::conversions::i256_from_bigint(&value)),
                        None => None
                    }
                }
            }
        }
        "H160" => {
            quote! {
                {
                    let value = common::database::from_cql::<Option<num_bigint::BigInt>>(columns.next(), #current_index, actual, EXPECTED)?;
                    match value {
                        Some(value) => Some(common::conversions::bigint_to_h160(value)),
                        None => None
                    }
                }
            }
        }
        "DateTime<Utc>" => {
            quote! {
                {
                    let value = common::database::from_cql::<Option<i64>>(columns.next(), #current_index, actual, EXPECTED)?;
                    match value {
                        Some(value) => Some(common::conversions::milliseconds_to_datetime_utc(value)),
                        None => None
                    }
                }
            }
        }
        "u32" => {
            quote! {
                {
                    let value = common::database::from_cql::<Option<i32>>(columns.next(), #current_index, actual, EXPECTED)?;
                    match value {
                        Some(value) => Some(value as u32),
                        None => None
                    }
                }
            }
        }
        "u64" => {
            quote! {
                {
                    let value = common::database::from_cql::<Option<i64>>(columns.next(), #current_index, actual, EXPECTED)?;
                    match value {
                        Some(value) => Some(value as u64),
                        None => None
                    }
                }
            }
        }
        _ => {
            if is_enum {
                let enum_ident: syn::Ident = syn::Ident::new(inner_type, Span::call_site());

                quote! {
                    {
                        let value = common::database::from_cql::<Option<String>>(columns.next(), #current_index, actual, EXPECTED)?;
                        match value {
                            Some(value) => Some(value.parse::<#enum_ident>().unwrap()),
                            None => None
                        }
                    }
                }
            } else {
                quote! {
                    common::database::from_cql(columns.next(), #current_index, actual, EXPECTED)?
                }
            }
        }
    }
}
