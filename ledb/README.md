# Lightweight embedded database

[![License: MIT](https://img.shields.io/badge/License-MIT-brightgreen.svg)](https://opensource.org/licenses/MIT)
[![Travis-CI Build Status](https://travis-ci.org/katyo/ledb.svg?branch=master)](https://travis-ci.org/katyo/ledb)
[![Appveyor Build status](https://ci.appveyor.com/api/projects/status/1wrmhivii22emfxg)](https://ci.appveyor.com/project/katyo/ledb)
[![Crates.io Package](https://img.shields.io/crates/v/ledb.svg?style=popout)](https://crates.io/crates/ledb)
[![Docs.rs API Documentation](https://docs.rs/ledb/badge.svg)](https://docs.rs/ledb)

The **LEDB** is an attempt to implement simple but efficient, lightweight but powerful document storage.

The abbreviation *LEDB* may be treated as an Lightweight Embedded DB, also Low End DB, also Literium Engine DB, also LitE DB, and so on.

## Links

* [ledb Crate on crates.io](https://crates.io/crates/ledb)
* [ledb API Docs on docs.rs](https://docs.rs/ledb)
* [ledb-types Crate on crates.io](https://crates.io/crates/ledb-types)
* [ledb-types API Docs on docs.rs](https://docs.rs/ledb-types)
* [ledb-derive Crate on crates.io](https://crates.io/crates/ledb-derive)
* [ledb-derive API Docs on docs.rs](https://docs.rs/ledb-derive)
* [ledb-actix Crate on crates.io](https://crates.io/crates/ledb-actix)
* [ledb-actix API Docs on docs.rs](https://docs.rs/ledb-actix)
* [ledb NodeJS addon on npmjs.com](https://npmjs.com/package/ledb)

## Key features

* Processing documents which implements `Serialize` and `Deserialize` traits from [serde](https://serde.rs/).
* Identifying documents using auto-incrementing integer primary keys.
* Indexing any fields of documents using unique or duplicated keys.
* Searching and ordering documents using indexed fields or primary key.
* Selecting documents using complex filters with fields comparing and logical operations.
* Updating documents using rich set of modifiers.
* Storing documents into independent storages so called collections.
* Flexible `query!` macro which helps write clear and readable queries.
* Using [LMDB](https://en.wikipedia.org/wiki/Lightning_Memory-Mapped_Database) as backend for document storage and indexing engine.

## Usage example

```rust
use serde::{Serialize, Deserialize};
use ledb::{Options, Storage, IndexKind, KeyType, Filter, Comp, Order, OrderKind, Primary};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Document)]
struct MyDoc {
    #[document(primary)]
    id: Option<Primary>,
    title: String,
    #[document(index)]
    tag: Vec<String>,
    #[document(unique)]
    timestamp: u32,
}

fn main() {
    let db_path = ".test_dbs/my_temp_db";
    let _ = std::fs::remove_dir_all(&db_path);

    // Open storage
    let storage = Storage::new(&db_path, Options::default()).unwrap();

    // Get collection
    let collection = storage.collection("my-docs").unwrap();

    // Ensure indexes
    query!(index for collection
        title str unique,
        tag str,
        timestamp int unique,
    ).unwrap();

    // Insert JSON document
    let first_id = query!(insert into collection {
        "title": "First title",
        "tag": ["some tag", "other tag"],
        "timestamp": 1234567890,
    }).unwrap();

    // Insert typed document
    let second_id = collection.insert(&MyDoc {
        title: "Second title".into(),
        tag: vec![],
        timestamp: 1234567657,
    }).unwrap();

    // Find documents
    let found_docs = query!(
        find MyDoc in collection
        where title == "First title"
    ).unwrap().collect::<Result<Vec<_>, _>>().unwrap();

    // Update documents
    let n_affected = query!(
        update in collection modify title = "Other title"
        where title == "First title"
    ).unwrap();

    // Find documents with descending ordering
    let found_docs = query!(
        find MyDoc in collection order desc
    ).unwrap().collect::<Result<Vec<_>, _>>().unwrap();

    // Remove documents
    let n_affected = query!(
        remove from collection where title == "Other title"
    ).unwrap();
}
```
