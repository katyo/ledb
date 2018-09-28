/*!

# LEDB Storage actor and REST interface

An implementation of storage actor for [Actix](https://actix.rs/).

*NOTE: Use `features = ["web"]` to enable an optional scoped REST-interface for **actix-web**.*

## Storage actor

Usage example:

```
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

#[macro_use]
extern crate log;
extern crate pretty_env_logger;

use actix::System;
use futures::Future;
use ledb_actix::{Storage, StorageAddrExt};
use serde_json::from_value;
use std::env;
use tokio::spawn;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct BlogPost {
    pub title: String,
    pub tags: Vec<String>,
    pub content: String,
}

fn main() {
    env::set_var("RUST_LOG", "info");
    pretty_env_logger::init().unwrap();

    let _ = std::fs::remove_dir_all("example_db");

    System::run(|| {
        let addr = Storage::new("example_db").unwrap().start(1);

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

## REST interface

*LEDB HTTP interface 0.1.0*

### Storage API

# get database info
GET /info
# get database statistics
GET /stats

### Collection API

# get list of collections
GET /collection
# create new empty collection
POST /collection?name=$collection_name
# drop collection with all documents
DELETE /collection/$collection_name

### Index API

# get indexes of collection
GET /collection/$collection_name/index
# create new index for collection
POST /collection/$collection_name/index?name=$field_path&kind=$index_kind&type=$key_type
# drop index of collection
DELETE /collection/$collection_name/document/$index_name

### Document API

# find documents using query
GET /collection/$collection_name/document?filter=$query&order=$ordering&offset=10&length=10
GET /collection/$collection_name?filter=$query&order=$ordering&offset=10&length=10
# modify documents using query
PUT /collection/$collection_name/document?filter=$query&modify=$modifications
PATCH /collection/$collection_name?filter=$query&modify=$modifications
# remove documents using query
DELETE /collection/$collection_name/document?filter=$query
PUT /collection/$collection_name?filter=$query

# insert new document
POST /collection/$collection_name/document
POST /collection/$collection_name
# get document by id
GET /collection/$collection_name/document/$document_id
GET /collection/$collection_name/$document_id
# replace document
PUT /collection/$collection_name/document/$document_id
PUT /collection/$collection_name/$document_id
# remove document
DELETE /collection/$collection_name/document/$document_id
DELETE /collection/$collection_name/$document_id

*/

extern crate actix;

#[allow(unused_imports)]
#[macro_use(_query_impl, _query_extr)]
extern crate ledb;

extern crate serde;

extern crate futures;

#[cfg(test)]
extern crate tokio;

#[cfg(any(test, feature = "web"))]
#[macro_use]
extern crate serde_derive;

#[cfg(test)]
#[macro_use]
extern crate serde_json;

#[cfg(feature = "web")]
extern crate serde_with;

#[cfg(feature = "web")]
extern crate actix_web;

mod actor;
mod extra;
mod macros;
#[cfg(feature = "web")]
mod scope;

pub use ledb::{
    Action, Comp, Cond, Document, DocumentsIterator, Filter, Identifier, IndexKind, Info, KeyData,
    KeyType, Modify, Order, OrderKind, Primary, Stats, Value,
};

pub use actor::*;
pub use extra::*;

#[cfg(feature = "web")]
pub use scope::*;
