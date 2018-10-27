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
// This allows inserting JSON documents
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
use ledb_actix::{Storage, Options, StorageAddrExt, Primary, Identifier, Document};
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
                    "id": 1,
                    "title": "Absurd",
                    "tags": ["absurd", "psychology"],
                    "content": "Still nothing..."
                })).unwrap();
                
                assert_eq!(&doc, &doc_data);
                assert!(docs.next().is_none());
                
                System::current().stop();
            }).map_err(|err| {
                error!("Error: {:?}", err);
            })
        );
    });
}
```

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

__POST__ /collection/_$collection_name_/index?path=_$field_name_&kind=_$index_kind_&key=_$key_type_

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
    KeyField, KeyFields, KeyType, Modify, Options, Order, OrderKind, Primary, Stats, Value,
};

pub use actor::*;
pub use extra::*;

#[cfg(feature = "web")]
pub use scope::*;
