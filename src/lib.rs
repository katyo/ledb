extern crate byteorder;
extern crate serde;
#[macro_use] extern crate serde_derive;
extern crate serde_cbor;
extern crate serde_json;
extern crate ron;
extern crate lmdb_zero as lmdb;
extern crate liblmdb_sys as lmdbffi;

mod error;
mod storage;
mod collection;
mod index;
mod filter;
mod document;

pub use error::{ResultWrap};
pub use document::{Primary, Document, Value};
pub use storage::{Storage};
pub use collection::{Collection};
pub use index::{Index, IndexKind};
pub use filter::{Filter, Comp, Cond, KeyType, KeyData};

#[cfg(test)]
mod test {
    use std::fs::remove_dir_all;
    use std::collections::HashSet;
    use super::{Storage, IndexKind, KeyType, Document, Filter, Comp, KeyData};

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
        
        assert_eq!(coll.get(i1).unwrap(), Some(Document::new_with_id(i1, u1)));

        assert_eq!(coll.find(Filter::Comp("name".into(), Comp::Eq(KeyData::String("kayo".into())))).unwrap(),
                   HashSet::new());

        //assert!(false);

        remove_dir_all(DB_DIR).unwrap();
    }
}
