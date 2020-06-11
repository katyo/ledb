use std::env;

use serde::{Deserialize, Serialize};
use serde_json::json;

use ledb_actix::{query, Document, Options, Primary, Storage, StorageAddrExt};
use log::info;
use serde_json::from_value;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Document)]
struct BlogPost {
    #[document(primary)]
    pub id: Option<Primary>,
    pub title: String,
    pub tags: Vec<String>,
    pub content: String,
}

#[actix_rt::main]
async fn main() {
    env::set_var("RUST_LOG", "info");
    pretty_env_logger::init();

    let _ = std::fs::remove_dir_all("example_db");

    let addr = Storage::new("example_db", Options::default())
        .unwrap()
        .start(1);

    let id = addr
        .send_query(query!(
            insert into blog {
                "title": "Absurd",
                "tags": ["absurd", "psychology"],
                "content": "Still nothing..."
            }
        ))
        .await
        .unwrap();

    info!("Inserted document id: {}", id);
    assert_eq!(id, 1);

    let id = addr.send_query(query!(
        insert into blog {
            "title": "Lorem ipsum",
            "tags": ["lorem", "ipsum"],
            "content": "Lorem ipsum dolor sit amet, consectetur adipiscing elit, sed do eiusmod tempor incididunt ut labore et dolore magna aliqua. Ut enim ad minim veniam, quis nostrud exercitation ullamco laboris nisi ut aliquip ex ea commodo consequat. Duis aute irure dolor in reprehenderit in voluptate velit esse cillum dolore eu fugiat nulla pariatur. Excepteur sint occaecat cupidatat non proident, sunt in culpa qui officia deserunt mollit anim id est laborum."
        }
    )).await.unwrap();

    info!("Inserted document id: {}", id);
    assert_eq!(id, 2);

    addr.send_query(query!(
        index for blog tags string
    ))
    .await
    .unwrap();

    info!("Indexing is ok");

    let mut docs = addr
        .send_query(query!(
            find BlogPost in blog
            where tags == "psychology"
                order asc
        ))
        .await
        .unwrap();

    info!("Number of found documents: {}", docs.size_hint().0);

    assert_eq!(docs.size_hint(), (1, Some(1)));

    let doc = docs.next().unwrap().unwrap();

    info!("Found document: {:?}", doc);

    let doc_data: BlogPost = from_value(json!({
        "id": 1,
        "title": "Absurd",
        "tags": ["absurd", "psychology"],
        "content": "Still nothing..."
    }))
    .unwrap();

    assert_eq!(&doc, &doc_data);
    assert!(docs.next().is_none());
}
