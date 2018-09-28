/*!

# LEDB Storage actor and REST interface

An implementation of storage actor for [Actix](https://actix.rs/).

*NOTE: Use `features = ["web"]` to enable an optional scoped REST-interface for **actix-web**.*

## Storage actor

Usage example:

```ignore
use futures::Future;
use tokio::spawn;
use actix::System;
use ledb_actix::{Storage, EnsureIndex, Insert, Find, IndexKind, KeyType};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct BlogPost {
    pub title: String,
    pub tags: Vec<String>,
    pub content: String,
}

static DB_PATH: &'static str = "test_db";

fn main() {
    System::run(|| {
        let storage = Storage::new(DB_PATH).unwrap();
        
        let addr = storage.start(4);

        let addr1 = addr.clone();
        let addr2 = addr.clone();
        let addr3 = addr.clone();
        
        spawn(
            addr.send(
                Insert::<_, BlogPost>("blog", json_val!({
                    "title": "Absurd",
                    "tags": ["absurd", "psychology"],
                    "content": "Still nothing..."
                }))
            ).and_then(move |res| {
                assert_eq!(res.unwrap(), 1);
                
                addr1.send(Insert::<_, BlogPost>("blog", json_val!({
                    "title": "Lorem ipsum",
                    "tags": ["lorem", "ipsum"],
                    "content": "Lorem ipsum dolor sit amet, consectetur adipiscing elit, sed do eiusmod tempor incididunt ut labore et dolore magna aliqua. Ut enim ad minim veniam, quis nostrud exercitation ullamco laboris nisi ut aliquip ex ea commodo consequat. Duis aute irure dolor in reprehenderit in voluptate velit esse cillum dolore eu fugiat nulla pariatur. Excepteur sint occaecat cupidatat non proident, sunt in culpa qui officia deserunt mollit anim id est laborum."
                })))
            }).and_then(move |res| {
                assert_eq!(res.unwrap(), 2);

                addr3.send(EnsureIndex("blog", "tags", IndexKind::Duplicate, KeyType::String))
            }).and_then(move |res| {
                assert!(res.is_ok());
                
                addr2.send(Find::<_, BlogPost>("blog",
                                json_val!({ "tags": { "$eq": "psychology" } }),
                                json_val!("$asc")))
            }).map(|res| {
                let mut docs = res.unwrap();
                assert_eq!(docs.size_hint(), (1, Some(1)));
                let doc = docs.next().unwrap().unwrap();
                let doc_data: BlogPost = json_val!({
                    "title": "Absurd",
                    "tags": ["absurd", "psychology"],
                    "content": "Still nothing..."
                });
                assert_eq!(doc.get_data(), &doc_data);
                assert!(docs.next().is_none());
                
                System::current().stop();
            }).map_err(|_| ())
        );
    });
}
```

# REST interface

*/

extern crate actix;

#[cfg(test)]
#[macro_use(_query_impl, _query_extr)]
extern crate ledb;

#[cfg(not(test))]
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
