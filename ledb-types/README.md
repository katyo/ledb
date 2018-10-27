# Types for defining storable documents

[![License: MIT](https://img.shields.io/badge/License-MIT-brightgreen.svg)](https://opensource.org/licenses/MIT)
[![Travis-CI Build Status](https://travis-ci.org/katyo/ledb.svg?branch=master)](https://travis-ci.org/katyo/ledb)
[![Appveyor Build status](https://ci.appveyor.com/api/projects/status/1wrmhivii22emfxg)](https://ci.appveyor.com/project/katyo/ledb)
[![Crates.io Package](https://img.shields.io/crates/v/ledb.svg?style=popout)](https://crates.io/crates/ledb)
[![Docs.rs API Documentation](https://docs.rs/ledb/badge.svg)](https://docs.rs/ledb)

This types and traits widely used for documents which can be managed using persistent storages like *LEDB*.

The **LEDB** is an attempt to implement simple but efficient, lightweight but powerful document storage.

The abbreviation *LEDB* may be treated as an Lightweight Embedded DB, also Low End DB, also Literium Engine DB, also LitE DB, and so on.

## Links

* [ledb-types Crate on crates.io](https://crates.io/crates/ledb-types)
* [ledb-types API Docs on docs.rs](https://docs.rs/ledb-types)
* [ledb-derive Crate on crates.io](https://crates.io/crates/ledb-derive)
* [ledb-derive API Docs on docs.rs](https://docs.rs/ledb-derive)
* [ledb Crate on crates.io](https://crates.io/crates/ledb)
* [ledb API Docs on docs.rs](https://docs.rs/ledb)

## Usage example

```rust
extern crate serde;
#[macro_use] extern crate serde_derive;
extern crate ledb_types;

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
