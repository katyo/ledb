extern crate byteorder;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate serde_cbor;
extern crate ron;
extern crate lmdb_zero as lmdb;
extern crate liblmdb_sys as lmdbffi;

#[cfg(test)]
#[macro_use]
extern crate serde_json;

#[cfg(test)]
#[macro_use]
mod test;

mod error;
mod document;
mod selection;
mod filter;
mod extra;
mod storage;
mod collection;
mod index;

pub use error::{Error, Result, ResultWrap};
pub use document::{Primary, Document, Value};
pub use storage::{Storage};
pub use collection::{Collection, DocumentsIterator};
pub use index::{Index, IndexKind};
pub use selection::{Selection};
pub use filter::{Filter, Comp, Cond, KeyType, KeyData, OrderKind, Order};

#[cfg(test)]
mod lib_test {
    use std::fs::remove_dir_all;
    use std::collections::HashSet;
    use serde_json::from_value;
    use super::{Storage, IndexKind, KeyType, Document, Filter, Comp, KeyData, Order, OrderKind};
    
    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
    pub struct UserData {
        pub name: String,
        pub hash: Option<Vec<u8>>,
        pub view: bool,
        pub prefs: UserPrefs,
    }

    impl Default for UserData {
        fn default() -> Self {
            Self { name: "".into(), hash: None, view: false, prefs: UserPrefs::default() }
        }
    }

    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
    pub struct UserPrefs {
        pub lang: Option<String>,
        pub zone: Option<i32>,
    }

    impl Default for UserPrefs {
        fn default() -> Self {
            Self { lang: None, zone: None }
        }
    }

    const DB_DIR: &'static str = ".test_db";

    #[test]
    fn test_open() {
        let s = Storage::open(DB_DIR).unwrap();

        let coll = s.collection("user").unwrap();
        coll.create_index("name", IndexKind::Unique, KeyType::String).unwrap();
        coll.create_index("view", IndexKind::Duplicate, KeyType::Int).unwrap();

        let mut u1 = UserData::default();
        u1.name = "kayo".into();

        let i1 = coll.insert(&u1).unwrap();

        assert_eq!(i1, 1);
        
        assert_eq!(coll.get(i1).unwrap(), Some(Document::new_with_id(i1, u1.clone())));

        assert_eq!(coll.find(None, Order::Primary(OrderKind::Asc))
                   .unwrap().collect::<Result<Vec<Document<UserData>>, _>>().unwrap(),
                   vec![Document::new_with_id(i1, u1.clone())]);

        // Some(Filter::Comp("name".into(), Comp::Eq(KeyData::String("kayo".into()))))
        // Order::ById(OrderKind::Asc)
        
        assert_eq!(coll.find(json_val!({"name":{"$eq":"kayo"}}), json_val!("$asc"))
                   .unwrap().collect::<Result<Vec<Document<UserData>>, _>>().unwrap(),
                   vec![Document::new_with_id(i1, u1.clone())]);

        let mut u2 = UserData::default();
        u2.name = "kiri".into();
        
        let i2 = coll.insert(&u2).unwrap();

        assert_eq!(i2, 2);
        
        assert_eq!(coll.get(i2).unwrap(), Some(Document::new_with_id(i2, u2.clone())));

        assert_eq!(coll.find(json_val!({"name":{"$eq":"kayo"}}), json_val!("$asc"))
                   .unwrap().collect::<Result<Vec<Document<UserData>>, _>>().unwrap(),
                   vec![Document::new_with_id(i1, u1.clone())]);

        assert_eq!(coll.find(json_val!(null), json_val!("$asc"))
                   .unwrap().collect::<Result<Vec<Document<UserData>>, _>>().unwrap(),
                   vec![Document::new_with_id(i1, u1.clone()),
                        Document::new_with_id(i2, u2.clone())]);

        assert_eq!(coll.find(json_val!(null), json_val!("$desc"))
                   .unwrap().collect::<Result<Vec<Document<UserData>>, _>>().unwrap(),
                   vec![Document::new_with_id(i2, u2.clone()),
                        Document::new_with_id(i1, u1.clone())]);

        //assert!(false);

        remove_dir_all(DB_DIR).unwrap();
    }
}
