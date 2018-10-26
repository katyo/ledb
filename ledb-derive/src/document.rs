use proc_macro2::{Span, TokenStream, TokenTree};
use syn::{Data, DeriveInput, Field, Fields, Lit, LitStr};

pub fn derive_document(input: &DeriveInput) -> Result<TokenStream, String> {
    let type_name = &input.ident;
    let mut primary_field: Option<String> = None;

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
                }
            }
            _ => return Err("Only struct with named fields can be represented as document".into()),
        },
        _ => return Err("Storable documents can be implemented using structs only".into()),
    }

    let primary_field = if let Some(primary_field) = primary_field {
        Lit::Str(LitStr::new(&primary_field.to_string(), Span::call_site()))
    } else {
        return Err("Document must contain primary key field".into());
    };

    Ok(quote! {
        impl ledb_types::Document for #type_name {
            fn primary_field() -> ledb_types::Identifier {
                #primary_field.into()
            }
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
                    match (&tts.next(), &tts.next(), &tts.next()) {
                        (
                            Some(TokenTree::Ident(name)),
                            Some(TokenTree::Punct(op)),
                            Some(TokenTree::Literal(val)),
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
                impl ledb_types::Document for TestDoc {
                    fn primary_field() -> ledb_types::Identifier {
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
                impl ledb_types::Document for TestDoc {
                    fn primary_field() -> ledb_types::Identifier {
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
                impl ledb_types::Document for TestDoc {
                    fn primary_field() -> ledb_types::Identifier {
                        "_id".into()
                    }
                }
            }.to_string()
        );
    }
}
