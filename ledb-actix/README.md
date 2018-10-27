# Actor and REST interface for LEDB

[![License: MIT](https://img.shields.io/badge/License-MIT-brightgreen.svg)](https://opensource.org/licenses/MIT)
[![Travis-CI Build Status](https://travis-ci.org/katyo/ledb.svg?branch=master)](https://travis-ci.org/katyo/ledb)
[![Appveyor Build status](https://ci.appveyor.com/api/projects/status/1wrmhivii22emfxg)](https://ci.appveyor.com/project/katyo/ledb)
[![Crates.io Package](https://img.shields.io/crates/v/ledb-actix.svg?style=popout)](https://crates.io/crates/ledb-actix)
[![Docs.rs API Documentation](https://docs.rs/ledb-actix/badge.svg)](https://docs.rs/ledb-actix)

The **LEDB** is an attempt to implement simple but efficient, lightweight but powerful document storage.

The abbreviation *LEDB* may be treated as an Lightweight Embedded DB, also Low End DB, also Literium Engine DB, also LitE DB, and so on.

## Links

* [ledb-actix Crate on crates.io](https://crates.io/crates/ledb-actix)
* [ledb-actix API Docs on docs.rs](https://docs.rs/ledb-actix)
* [ledb Crate on crates.io](https://crates.io/crates/ledb)
* [ledb API Docs on docs.rs](https://docs.rs/ledb)
* [ledb-types Crate on crates.io](https://crates.io/crates/ledb-types)
* [ledb-types API Docs on docs.rs](https://docs.rs/ledb-types)
* [ledb-derive Crate on crates.io](https://crates.io/crates/ledb-derive)
* [ledb-derive API Docs on docs.rs](https://docs.rs/ledb-derive)

## REST-interface

*LEDB HTTP interface 0.1.0*

### Storage API

#### get database info

__GET__ /info

#### get database statistics

__GET__ /stats

### Collection API

#### get list of collections

__GET__ /collection

#### create new empty collection

__POST__ /collection?name=_$collection_name_

#### drop collection with all documents

__DELETE__ /collection/_$collection_name_

### Index API

#### get indexes of collection

__GET__ /collection/_$collection_name_/index

#### create new index for collection

__POST__ /collection/_$collection_name_/index?name=_$field_name_&kind=_$index_kind_&type=_$key_type_

#### drop index of collection

__DELETE__ /collection/_$collection_name_/document/_$index_name_

### Document API

#### find documents using query

__GET__ /collection/_$collection_name_/document?filter=_$query_&order=_$ordering_&offset=_$skip_&length=_$take_

__GET__ /collection/_$collection_name_?filter=_$query_&order=_$ordering_&offset=_$skip_&length=_$take_

#### modify documents using query

__PUT__ /collection/_$collection_name_/document?filter=_$query_&modify=_$modifications_

__PATCH__ /collection/_$collection_name_?filter=_$query_&modify=_$modifications_

#### remove documents using query

__DELETE__ /collection/_$collection_name_/document?filter=_$query_

__PUT__ /collection/_$collection_name_?filter=_$query_

#### insert new document

__POST__ /collection/_$collection_name_/document

__POST__ /collection/_$collection_name_

#### get document by id

__GET__ /collection/_$collection_name_/document/_$document_id_

__GET__ /collection/_$collection_name_/_$document_id_

#### replace document

__PUT__ /collection/_$collection_name_/document/_$document_id_

__PUT__ /collection/_$collection_name_/_$document_id_

#### remove document

__DELETE__ /collection/_$collection_name_/document/_$document_id_

__DELETE__ /collection/_$collection_name_/_$document_id_

### Supported index kinds

* uni -- Unique key
* dup -- Duplicated keys

### Supported key types

* int    -- 64-bit signed integer
* float  -- 64-bit floating point number
* bool   -- boolean value
* string -- UTF-8 string
* binary -- binary data

## Actor

### Usage example

```rust
extern crate actix;
extern crate futures;
extern crate tokio;

#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate serde_json;
#[macro_use]
extern crate ledb;
#[macro_use]
extern crate ledb_actix;
// This allows define typed documents easy
#[macro_use]
extern crate ledb_derive;
extern crate ledb_types;

#[macro_use]
extern crate log;
extern crate pretty_env_logger;

use actix::System;
use futures::Future;
use ledb_actix::{Options, Storage, StorageAddrExt, Primary};
use serde_json::from_value;
use std::env;
use tokio::spawn;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Document)]
struct BlogPost {
    #[document(primary)]
    pub id: Option<Primary>,
    pub title: String,
    pub tags: Vec<String>,
    pub content: String,
}

fn main() {
    env::set_var("RUST_LOG", "info");
    pretty_env_logger::init().unwrap();

    let _ = std::fs::remove_dir_all("example_db");

    System::run(|| {
        let addr = Storage::new("example_db", Options::default()).unwrap().start(1);

        spawn(
            addr.clone().send_query(query!(
                insert into blog {
                    "title": "Absurd",
                    "tags": ["absurd", "psychology"],
                    "content": "Still nothing..."
                }
            )).and_then({ let addr = addr.clone(); move |id| {
                info!("Inserted document id: {}", id);
                assert_eq!(id, 1);
                
                addr.send_query(query!(
                    insert into blog {
                        "title": "Lorem ipsum",
                        "tags": ["lorem", "ipsum"],
                        "content": "Lorem ipsum dolor sit amet, consectetur adipiscing elit, sed do eiusmod tempor incididunt ut labore et dolore magna aliqua. Ut enim ad minim veniam, quis nostrud exercitation ullamco laboris nisi ut aliquip ex ea commodo consequat. Duis aute irure dolor in reprehenderit in voluptate velit esse cillum dolore eu fugiat nulla pariatur. Excepteur sint occaecat cupidatat non proident, sunt in culpa qui officia deserunt mollit anim id est laborum."
                    }
                ))
            } }).and_then({ let addr = addr.clone(); move |id| {
                info!("Inserted document id: {}", id);
                assert_eq!(id, 2);

                addr.send_query(query!(
                    index for blog tags string
                ))
            } }).and_then({ let addr = addr.clone(); move |_| {
                info!("Indexing is ok");
                
                addr.send_query(query!(
                    find BlogPost in blog
                    where tags == "psychology"
                        order asc
                ))
            } }).map(|mut docs| {
                info!("Number of found documents: {}", docs.size_hint().0);
                
                assert_eq!(docs.size_hint(), (1, Some(1)));
                
                let doc = docs.next().unwrap().unwrap();

                info!("Found document: {:?}", doc);
                
                let doc_data: BlogPost = from_value(json!({
                    "title": "Absurd",
                    "tags": ["absurd", "psychology"],
                    "content": "Still nothing..."
                })).unwrap();
                
                assert_eq!(doc.get_data(), &doc_data);
                assert!(docs.next().is_none());
                
                System::current().stop();
            }).map_err(|err| {
                error!("Error: {:?}", err);
            })
        );
    });
}
```
