/*!

# Types and traits for storable documents

## Document trait

The basic trait which should be implemented for structs which designed to be handled as documents.

```rust
# extern crate serde;
# extern crate ledb_types;
#
use serde::{Serialize, Deserialize};
use ledb_types::{Document, Identifier, Primary, KeyFields, KeyType, IndexKind};

#[derive(Serialize, Deserialize)]
struct MyDoc {
    // define optional primary key field
    id: Option<Primary>,
    // define other fields
    title: String,
    tag: Vec<String>,
    timestamp: u32,
    // define nested document
    meta: MetaData,
}

#[derive(Serialize, Deserialize)]
struct MetaData {
    // define index field
    keywords: Vec<String>,
    // define other fields
    description: String,
}

impl Document for MyDoc {
    // declare primary key field name
    fn primary_field() -> Identifier {
        "id".into()
    }

    // declare other key fields
    fn key_fields() -> KeyFields {
        KeyFields::new()
            // add key fields of document
            .with_field(("title", KeyType::String, IndexKind::Unique))
            .with_field(("tag", KeyType::String, IndexKind::Index))
            .with_field(("timestamp", KeyType::Int, IndexKind::Unique))
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

## DocumentKeyType trait

This trait maps rust types to key types.

*/

extern crate serde;

#[cfg(feature = "json")]
extern crate serde_json;

#[cfg(feature = "cbor")]
extern crate serde_cbor;

#[cfg(feature = "bytes")]
extern crate bytes;

mod document;
mod identifier;
mod index;

pub use self::document::*;
pub use self::identifier::*;
pub use self::index::*;
