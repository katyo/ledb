use proc_macro2::{Span, TokenStream, TokenTree};
use syn::{Data, DeriveInput, Field, Fields, Lit, LitStr, Type};

pub fn derive_document(input: &DeriveInput) -> Result<TokenStream, String> {
    let type_name = &input.ident;
    let is_nested = has_nested_attribute(input);
    let mut primary_field = None;
    let mut index_fields = Vec::new();
    let mut nested_docs = Vec::new();

    match &input.data {
        Data::Struct(data) => match &data.fields {
            Fields::Named(fields) => {
                for field in &fields.named {
                    if let Some(primary_field_name) = get_primary_attribute(&field) {
                        if primary_field.is_none() {
                            primary_field = get_serde_rename(&field).or(Some(primary_field_name));
                        } else {
                            return Err(format!("Only one primary key field per document allowed"));
                        }
                    }

                    if let Some((field_name, field_type, index_kind)) = get_index_attribute(&field)
                    {
                        index_fields.push((
                            get_serde_rename(&field).unwrap_or(field_name),
                            field_type,
                            index_kind,
                        ));
                    }

                    if let Some((field_name, field_type)) = get_nested_attribute(&field) {
                        nested_docs
                            .push((get_serde_rename(&field).unwrap_or(field_name), field_type));
                    }
                }
            }
            _ => return Err("Only struct with named fields can be represented as document".into()),
        },
        _ => return Err("Storable documents can be implemented using structs only".into()),
    }

    let primary_field_fn = if let Some(primary_field) = primary_field {
        let primary_field = Lit::Str(LitStr::new(&primary_field.to_string(), Span::call_site()));
        quote! {
            fn primary_field() -> ::ledb_types::Identifier {
                #primary_field.into()
            }
        }
    } else {
        // We don't require primary key for nested documents
        if is_nested {
            TokenStream::new()
        } else {
            return Err("Document must contain primary key field".into());
        }
    };

    let key_fields_fn = if index_fields.is_empty() && nested_docs.is_empty() {
        TokenStream::new()
    } else {
        let index_fields = index_fields
            .into_iter()
            .map(|(field_name, field_type, index_kind)| {
                let field_name = Lit::Str(LitStr::new(&field_name, Span::call_site()));
                let field_type = match field_type {
                    Ok(field_type) => quote! { <#field_type as ::ledb_types::DocumentKeyType>::key_type() },
                    Err(key_type) => quote! { ::ledb_types::KeyType::#key_type },
                };
                let index_kind = match index_kind.as_str() {
                    "unique" => quote! { ::ledb_types::IndexKind::Unique },
                    "index" => quote! { ::ledb_types::IndexKind::Index },
                    _ => unreachable!(),
                };

                quote! {
                    (#field_name, #field_type, #index_kind)
                }
            });

        let nested_docs = nested_docs
            .into_iter()
            .map(|(field_name, field_type)| {
                let field_name = Lit::Str(LitStr::new(&field_name, Span::call_site()));
                
                quote! {
                    <#field_type as ::ledb_types::Document>::key_fields().with_parent(#field_name)
                }
            });

        quote! {
            fn key_fields() -> ::ledb_types::KeyFields {
                ::ledb_types::KeyFields::new()
                    #(.with_field(#index_fields))*
                    #(.with_fields(#nested_docs))*
            }
        }
    };

    Ok(quote! {
        impl ::ledb_types::Document for #type_name {
            #primary_field_fn
            #key_fields_fn
        }
    })
}

fn get_primary_attribute(field: &Field) -> Option<String> {
    if let Some(ident) = &field.ident {
        for attr in &field.attrs {
            if attr.path.leading_colon.is_none()
                && attr.path.segments.len() == 1
                && attr.path.segments.first().unwrap().value().ident == "document"
                && attr.tts.clone().into_iter().any(|tt| {
                    if let TokenTree::Group(group) = tt {
                        group.stream().into_iter().any(|tt| {
                            if let TokenTree::Ident(ident) = tt {
                                ident == "primary"
                            } else {
                                false
                            }
                        })
                    } else {
                        false
                    }
                }) {
                return Some(ident.to_string());
            }
        }
    }

    None
}

fn get_index_attribute(field: &Field) -> Option<(String, Result<Type, TokenStream>, String)> {
    if let Some(ident) = &field.ident {
        for attr in &field.attrs {
            if attr.path.leading_colon.is_none()
                && attr.path.segments.len() == 1
                && attr.path.segments.first().unwrap().value().ident == "document"
            {
                for tt in attr.tts.clone() {
                    if let TokenTree::Group(group) = tt {
                        let mut tts = group.stream().into_iter();
                        match &tts.next() {
                            Some(TokenTree::Ident(kind))
                                if kind == "unique" || kind == "index" => {
                                    let key_type = if let Some(TokenTree::Ident(key)) = &tts.next() {
                                        match key.to_string().as_ref() {
                                            "int" | "integer" => Err(quote!(Int)),
                                            "float" => Err(quote!(Float)),
                                            "str" | "string" => Err(quote!(String)),
                                            "bin" | "binary" => Err(quote!(Binary)),
                                            "bool" | "boolean" => Err(quote!(Bool)),
                                            _ => Ok(field.ty.clone()),
                                        }
                                    } else {
                                        Ok(field.ty.clone())
                                    };
                                    
                                    return Some((
                                        ident.to_string(),
                                        key_type,
                                        kind.to_string(),
                                    ));
                                },
                            _ => (),
                        }
                    }
                }
            }
        }
    }

    None
}

fn has_nested_attribute(input: &DeriveInput) -> bool {
    for attr in &input.attrs {
        if attr.path.leading_colon.is_none()
            && attr.path.segments.len() == 1
            && attr.path.segments.first().unwrap().value().ident == "document"
        {
            for tt in attr.tts.clone() {
                if let TokenTree::Group(group) = tt {
                    let mut tts = group.stream().into_iter();
                    match (&tts.next(), &tts.next()) {
                        (Some(TokenTree::Ident(kind)), None) if kind == "nested" => {
                            return true;
                        }
                        _ => (),
                    }
                }
            }
        }
    }

    false
}

fn get_nested_attribute(field: &Field) -> Option<(String, Type)> {
    if let Some(ident) = &field.ident {
        for attr in &field.attrs {
            if attr.path.leading_colon.is_none()
                && attr.path.segments.len() == 1
                && attr.path.segments.first().unwrap().value().ident == "document"
            {
                for tt in attr.tts.clone() {
                    if let TokenTree::Group(group) = tt {
                        let mut tts = group.stream().into_iter();
                        match (&tts.next(), &tts.next()) {
                            (Some(TokenTree::Ident(kind)), None) if kind == "nested" => {
                                return Some((ident.to_string(), field.ty.clone()));
                            }
                            _ => (),
                        }
                    }
                }
            }
        }
    }

    None
}

fn get_serde_rename(field: &Field) -> Option<String> {
    let mut field_name = None;

    for attr in &field.attrs {
        if attr.path.leading_colon.is_none()
            && attr.path.segments.len() == 1
            && attr.path.segments.first().unwrap().value().ident == "serde"
        {
            for tt in attr.tts.clone() {
                if let TokenTree::Group(group) = tt {
                    let mut tts = group.stream().into_iter();
                    match (&tts.next(), &tts.next(), &tts.next(), &tts.next()) {
                        (
                            Some(TokenTree::Ident(name)),
                            Some(TokenTree::Punct(op)),
                            Some(TokenTree::Literal(val)),
                            None,
                        )
                            if name == "rename" && op.as_char() == '=' =>
                        {
                            if let Lit::Str(name) = Lit::new(val.clone()) {
                                field_name = Some(name.value());
                            }
                        }
                        _ => (),
                    }
                }
            }
        }
    }

    field_name
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn document_primary() {
        let src: DeriveInput = parse_quote! {
            #[derive(Document)]
            struct TestDoc {
                #[document(primary)]
                id: Option<Primary>,
            }
        };

        let res = derive_document(&src).unwrap();

        assert_eq!(
            res.to_string(),
            quote! {
                impl ::ledb_types::Document for TestDoc {
                    fn primary_field() -> ::ledb_types::Identifier {
                        "id".into()
                    }
                }
            }.to_string()
        );
    }

    #[test]
    fn document_primary_missing() {
        let src: DeriveInput = parse_quote! {
            #[derive(Document)]
            struct TestDoc {
                id: Option<Primary>,
            }
        };

        let res = derive_document(&src).unwrap_err();

        assert_eq!(res, "Document must contain primary key field");
    }

    #[test]
    fn document_primary_multiple() {
        let src: DeriveInput = parse_quote! {
            #[derive(Document)]
            struct TestDoc {
                #[document(primary)]
                id: Option<Primary>,
                #[document(primary)]
                key: Option<Primary>,
            }
        };

        let res = derive_document(&src).unwrap_err();

        assert_eq!(res, "Only one primary key field per document allowed");
    }

    #[test]
    fn document_primary_non_struct() {
        let src: DeriveInput = parse_quote! {
            #[derive(Document)]
            enum TestDoc {
                VarA,
                VarB,
            }
        };

        let res = derive_document(&src).unwrap_err();

        assert_eq!(
            res,
            "Storable documents can be implemented using structs only"
        );
    }

    #[test]
    fn document_primary_not_named() {
        let src: DeriveInput = parse_quote! {
            #[derive(Document)]
            struct TestDoc (
                #[document(primary)]
                Option<Primary>,
            );
        };

        let res = derive_document(&src).unwrap_err();

        assert_eq!(
            res,
            "Only struct with named fields can be represented as document"
        );
    }

    #[test]
    fn document_primary_serde_rename_before() {
        let src: DeriveInput = parse_quote! {
            #[derive(Document)]
            struct TestDoc {
                #[serde(rename = "_id")]
                #[document(primary)]
                id: Option<Primary>,
            }
        };

        let res = derive_document(&src).unwrap();

        assert_eq!(
            res.to_string(),
            quote! {
                impl ::ledb_types::Document for TestDoc {
                    fn primary_field() -> ::ledb_types::Identifier {
                        "_id".into()
                    }
                }
            }.to_string()
        );
    }

    #[test]
    fn document_primary_serde_rename_after() {
        let src: DeriveInput = parse_quote! {
            #[derive(Document)]
            struct TestDoc {
                #[document(primary)]
                #[serde(rename = "_id")]
                id: Option<Primary>,
            }
        };

        let res = derive_document(&src).unwrap();

        assert_eq!(
            res.to_string(),
            quote! {
                impl ::ledb_types::Document for TestDoc {
                    fn primary_field() -> ::ledb_types::Identifier {
                        "_id".into()
                    }
                }
            }.to_string()
        );
    }

    #[test]
    fn document_key_fields() {
        let src: DeriveInput = parse_quote! {
            #[derive(Document)]
            struct TestDoc {
                #[document(primary)]
                id: u32,
                #[document(unique)]
                title: String,
                #[document(index)]
                tags: Vec<String>,
                #[document(unique)]
                #[serde(rename = "created")]
                timestamp: i64,
                #[document(index binary)]
                hash: Vec<u8>,
            }
        };

        let res = derive_document(&src).unwrap();

        assert_eq!(
            res.to_string(),
            quote! {
                impl ::ledb_types::Document for TestDoc {
                    fn primary_field() -> ::ledb_types::Identifier {
                        "id".into()
                    }
                    
                    fn key_fields() -> ::ledb_types::KeyFields {
                        ::ledb_types::KeyFields::new()
                            .with_field(("title", <String as ::ledb_types::DocumentKeyType>::key_type(), ::ledb_types::IndexKind::Unique))
                            .with_field(("tags", <Vec<String> as ::ledb_types::DocumentKeyType>::key_type(), ::ledb_types::IndexKind::Index))
                            .with_field(("created", <i64 as ::ledb_types::DocumentKeyType>::key_type(), ::ledb_types::IndexKind::Unique))
                            .with_field(("hash", ::ledb_types::KeyType::Binary, ::ledb_types::IndexKind::Index))
                    }
                }
            }.to_string()
        );
    }

    #[test]
    fn document_nested() {
        let src: DeriveInput = parse_quote! {
            #[derive(Document)]
            #[document(nested)]
            struct TestDoc {
                #[document(unique)]
                field: String,
            }
        };

        let res = derive_document(&src).unwrap();

        assert_eq!(
            res.to_string(),
            quote! {
                impl ::ledb_types::Document for TestDoc {
                    fn key_fields() -> ::ledb_types::KeyFields {
                        ::ledb_types::KeyFields::new()
                            .with_field(("field", <String as ::ledb_types::DocumentKeyType>::key_type(), ::ledb_types::IndexKind::Unique))
                    }
                }
            }.to_string()
        );
    }

    #[test]
    fn document_nested_nested() {
        let src1: DeriveInput = parse_quote! {
            #[derive(Document)]
            struct TestDoc {
                #[document(primary)]
                id: Option<Primary>,
                #[document(unique)]
                field: String,
                #[document(nested)]
                nested: Vec<NestedDoc>,
            }
        };

        let src2: DeriveInput = parse_quote! {
            #[derive(Document)]
            #[document(nested)]
            struct NestDoc {
                #[document(index)]
                field: i32,
            }
        };

        let res1 = derive_document(&src1).unwrap();
        let res2 = derive_document(&src2).unwrap();

        assert_eq!(
            res1.to_string(),
            quote! {
                impl ::ledb_types::Document for TestDoc {
                    fn primary_field() -> ::ledb_types::Identifier {
                        "id".into()
                    }
                    
                    fn key_fields() -> ::ledb_types::KeyFields {
                        ::ledb_types::KeyFields::new()
                            .with_field(("field", <String as ::ledb_types::DocumentKeyType>::key_type(), ::ledb_types::IndexKind::Unique))
                            .with_fields(<Vec<NestedDoc> as ::ledb_types::Document>::key_fields().with_parent("nested"))
                    }
                }
            }.to_string()
        );

        assert_eq!(
            res2.to_string(),
            quote! {
                impl ::ledb_types::Document for NestDoc {
                    fn key_fields() -> ::ledb_types::KeyFields {
                        ::ledb_types::KeyFields::new()
                            .with_field(("field", <i32 as ::ledb_types::DocumentKeyType>::key_type(), ::ledb_types::IndexKind::Index))
                    }
                }
            }.to_string()
        );
    }
}
