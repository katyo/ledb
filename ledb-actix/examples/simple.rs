extern crate actix;
extern crate futures;
extern crate tokio;

extern crate serde;
extern crate serde_json;
extern crate ledb_types;
extern crate ledb;
extern crate ledb_actix;

extern crate log;
extern crate pretty_env_logger;

use serde::{Serialize, Deserialize};
use serde_json::{json};

use log::{info, error};
use actix::System;
use futures::Future;
use ledb_actix::{Storage, Options, StorageAddrExt, Primary, Document, query, query_extr};
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
    pretty_env_logger::init();

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
    }).unwrap();
}
