/*!

# Derive macro for defining storable documents

This crate helps to turn rust structures into documents which can be stored, indexed and queried.

## Defining documents

You may turn any struct into a document using `Document` in derive annotation like this:

```ignore
#[derive(Serialize, Deserialize, Document)]
struct MyDoc {
    // primary field
    #[document(primary)]
    id: Option<u32>,
    // other fields
}
```

This generates `Document` trait implementation for struct `MyDoc`.
It requires single field marked as primary key per document.
Currently primary key should be an integer only.
Also it not needed to be an optional field, but in this case you should take care of parsing (for example add `serde(default)` annotation).

## Defining key fields for indexing

To turn document field into key you can add document index annotation to it:

```ignore
#[derive(Serialize, Deserialize, Document)]
struct MyDoc {
    // primary field
    #[serde(default)]
    #[document(primary)]
    id: u32,
    // unique string key
    #[document(unique)]
    title: String,
    // normal string index
    #[document(index)]
    keywords: Vec<String>,
    // unique int key
    #[document(unique)]
    timestamp: u64,
}
```

## Overriding key types

In some cases it may be ambiguous to determine actual type of key by field type.
For example, when you try to index binary data using `Vec<u8>`, the actually determined key type is an integer (u8).
So you required to override key type manually using annotation like so:

```ignore
#[derive(Serialize, Deserialize, Document)]
struct MyDoc {
    // ...
    #[document(unique binary)]
    #[serde(with = "bytes")]
    hash: Vec<u8>,
}
```

## Nested documents

Of course you can add nested documents which may also have key fields:

```ignore
#[derive(Serialize, Deserialize, Document)]
struct MyDoc {
    // primary field
    #[document(primary)]
    #[serde(default)]
    id: u32,
    // ...fields
    // simple nested document
    #[document(nested)]
    meta: NestedDoc,
    // list of nested documents
    #[document(nested)]
    links: Vec<Link>,
    // map of nested documents
    #[document(nested)]
    props: HashMap<String, Prop>,
}
```

The primary key field may be omitted for nested documents only.
The nested documents should be marked as nested explicitly:

```ignore
#[derive(Serialize, Deserialize, Document)]
#[document(nested)]
struct NestedDoc {
    // ...fields
}
```

## Simple usage example

```ignore
extern crate serde;
#[macro_use]
extern crate serde_derive;

extern crate ledb_types;
#[macro_use]
extern crate ledb_derive;

#[derive(Serialize, Deserialize, Document)]
struct MyDoc {
    // define optional primary key field
    #[document(primary)]
    id: Option<Primary>,
    // define unique key field
    #[document(unique)]
    title: String,
    // define index fields
    #[document(index)]
    tag: Vec<String>,
    #[document(unique)]
    timestamp: u32,
    // define nested document
    #[document(nested)]
    meta: MetaData,
}

#[derive(Serialize, Deserialize, Document)]
#[document(nested)]
struct MetaData {
    // define index field
    #[document(index)]
    keywords: Vec<String>,
    // define other fields
    description: String,
}
```

It will generate the `Document` traits like so:

```ignore
impl Document for MyDoc {
    // declare primary key field name
    fn primary_field() -> Identifier {
        "id".into()
    }
    
    // declare other key fields for index
    fn key_fields() -> KeyFields {
        KeyFields::new()
            // add key fields of document
            .with_field(("title", String::key_type(), IndexKind::Unique))
            .with_field(("tag", String::key_type(), IndexKind::Index))
            .with_field(("timestamp", u32::key_type(), IndexKind::Unique))
            // add key fields from nested document
            .with_fields(MetaData::key_fields().with_parent("meta"))
    }
}

impl Document for MetaData {
    // declare key fields for index
    fn key_fields() -> KeyFields {
        KeyFields::new()
            // add key fields of document
            .with_field(("keywords", KeyType::String, IndexKind::Index))
    }
}
```

*/

#![recursion_limit = "128"]
extern crate proc_macro;
extern crate proc_macro2;
#[macro_use]
extern crate syn;
#[macro_use]
extern crate quote;

mod document;

use proc_macro::TokenStream;
use syn::DeriveInput;

#[proc_macro_derive(Document, attributes(document))]
pub fn derive_document(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    document::derive_document(&input)
        .unwrap_or_else(compile_error)
        .into()
}

fn compile_error(message: String) -> proc_macro2::TokenStream {
    quote! {
        compile_error!(#message);
    }
}
