extern crate byteorder;
extern crate serde;
#[macro_use] extern crate serde_derive;
extern crate serde_cbor;
extern crate serde_json;
extern crate ron;
extern crate lmdb_zero as lmdb;
extern crate liblmdb_sys as lmdbffi;

mod types;
mod storage;
mod collection;
mod index;
mod filter;

pub use types::{Id, Document, Binary, ResultWrap, NOT_FOUND};
pub use storage::{Storage};
pub use collection::{Collection};
pub use index::{Index, IndexKind, IndexType};
pub use filter::{Filter, Atom, Comp, Cond};

#[cfg(test)]
mod test {
    use std::fs::remove_dir_all;
    use super::{Id, Storage, IndexKind, IndexType};

    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
    pub struct UserData {
        pub _id: Option<Id>,
        pub name: String,
        pub hash: Option<Vec<u8>>,
        pub view: bool,
        pub prefs: UserPrefs,
    }

    impl Default for UserData {
        fn default() -> Self {
            Self { _id: None, name: "".into(), hash: None, view: false, prefs: UserPrefs::default() }
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
        coll.create_index("name", IndexKind::Unique, IndexType::String).unwrap();
        coll.create_index("view", IndexKind::Duplicate, IndexType::UInt).unwrap();

        let mut u1 = UserData::default();
        u1.name = "kayo".into();

        let i1 = coll.insert(&u1).unwrap();

        assert_eq!(i1, 1);

        let mut u1_ = u1.clone();
        u1_._id = Some(1);
        
        assert_eq!(coll.get(i1).unwrap(), Some(u1_));

        assert!(false);

        remove_dir_all(DB_DIR).unwrap();
    }
}
