# Lightweight embedded database

[![License: MIT](https://img.shields.io/badge/License-MIT-brightgreen.svg)](https://opensource.org/licenses/MIT)
[![Travis-CI Build Status](https://travis-ci.org/katyo/ledb.svg?branch=master)](https://travis-ci.org/katyo/ledb)
[![Appveyor Build status](https://ci.appveyor.com/api/projects/status/1wrmhivii22emfxg)](https://ci.appveyor.com/project/katyo/ledb)

The **LEDB** is an attempt to implement simple but efficient, lightweight but powerful document storage.

The abbreviation *LEDB* may be treated as an Lightweight Embedded DB, also Low End DB, also Literium Engine DB, also LitE DB, and so on.

## Links

* [ledb Crate on crates.io](https://crates.io/crates/ledb)
* [ledb API Docs on docs.rs](https://docs.rs/ledb)

## Features

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
extern crate serde;
#[macro_use] extern crate serde_derive;
// This allows inserting JSON documents
#[macro_use] extern crate serde_json;
#[macro_use] extern crate ledb;

use ledb::{Storage, IndexKind, KeyType, Filter, Comp, Order, OrderKind};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct MyDoc {
    title: String,
    tag: Vec<String>,
    timestamp: u32,
}

fn main() {
    let db_path = ".test_dbs/my_temp_db";
    let _ = std::fs::remove_dir_all(&db_path);

    // Open storage
    let storage = Storage::new(&db_path).unwrap();
    
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
