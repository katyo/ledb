extern crate byteorder;
extern crate ordered_float;
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
mod float;
mod value;
mod document;
mod selection;
mod filter;
mod extra;
mod storage;
mod collection;
mod index;

pub use error::{Error, Result, ResultWrap};
pub use document::{Primary, Document, Value};
pub use storage::{Storage, DatabaseDef};
pub use collection::{Collection, CollectionDef, DocumentsIterator};
pub use index::{Index, IndexKind, IndexDef};
pub use selection::{Selection};
pub use value::{KeyType, KeyData};
pub use filter::{Filter, Comp, Cond, OrderKind, Order};

#[cfg(test)]
mod tests {
    use serde_json::from_value;
    use test::test_db;
    use super::{Value, Collection, IndexKind, KeyType, Document, Result};

    macro_rules! assert_found {
        ($res:expr $(,$exp:expr)*) => {
            assert_eq!($res.unwrap().map(|doc: Result<Document<Value>>| doc.unwrap().get_id().unwrap()).collect::<Vec<_>>(), vec![$($exp),*])
        }
    }
    
    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
    pub struct Doc {
        pub s: String,
        pub b: bool,
        pub i: Vec<i32>,
        pub f: Option<f32>,
        pub n: Option<SubDoc>,
    }

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
    pub struct SubDoc {
        pub i: i64,
        pub a: Vec<String>,
    }
    
    fn fill_data(c: &Collection) -> Result<()> {
        c.insert(&json!({ "s": "abc", "b": false, "i": [] }))?;
        c.insert(&json!({ "s": "def", "b": false, "i": [1, 2], "n": { "i": 1, "a": ["t1", "t2"] } }))?;
        c.insert(&json!({ "s": "123", "b": true, "i": [2, 3, 4], "n": { "i": 2, "a": [] } }))?;
        c.insert(&json!({ "s": "xyz", "b": true, "i": [1, 5, 4], "n": { "i": 3, "a": ["t2", "t4"] } }))?;
        c.insert(&json!({ "s": "321", "b": false, "i": [2], "n": { "i": 2, "a": ["t4", "t1"] } }))?;
        c.insert(&json!({ "s": "456", "b": true, "i": [3, 5], "n": { "i": -11, "a": ["t2"] } }))?;
        Ok(())
    }

    fn mk_index(c: &Collection) -> Result<()> {
        c.create_index("s", IndexKind::Unique, KeyType::String)?;
        c.create_index("b", IndexKind::Duplicate, KeyType::Bool)?;
        c.create_index("i", IndexKind::Duplicate, KeyType::Int)?;
        c.create_index("f", IndexKind::Duplicate, KeyType::Float)?;
        c.create_index("n.i", IndexKind::Duplicate, KeyType::Int)?;
        c.create_index("n.a", IndexKind::Duplicate, KeyType::String)?;
        Ok(())
    }

    #[test]
    fn insert_documents() {
        let s = test_db("insert").unwrap();
        let c = s.collection("test").unwrap();

        assert_eq!(c.insert(&Doc::default()).unwrap(), 1);
        assert_eq!(c.insert(&Doc::default()).unwrap(), 2);
        assert_eq!(c.insert(&Doc::default()).unwrap(), 3);
    }

    #[test]
    fn get_by_primary() {
        let s = test_db("get").unwrap();
        let c = s.collection("test").unwrap();

        fill_data(&c).unwrap();

        assert_eq!(&c.get::<Doc>(1).unwrap().unwrap().s, "abc");
        assert_eq!(&c.get::<Doc>(2).unwrap().unwrap().s, "def");
        assert_eq!(&c.get::<Doc>(5).unwrap().unwrap().s, "321");
    }

    #[test]
    fn index_before_insert() {
        let s = test_db("index_before_insert").unwrap();
        let c = s.collection("test").unwrap();

        mk_index(&c).unwrap();
        fill_data(&c).unwrap();

        assert_eq!(&c.find_all::<Doc>(json_val!({ "s": { "$eq": "abc" } }), json_val!("$asc")).unwrap().get(0).unwrap().s, "abc");
    }

    #[test]
    fn index_after_insert() {
        let s = test_db("index_after_insert").unwrap();
        let c = s.collection("test").unwrap();

        fill_data(&c).unwrap();
        mk_index(&c).unwrap();

        assert_eq!(&c.find_all::<Doc>(json_val!({ "s": { "$eq": "abc" } }), json_val!("$asc")).unwrap().get(0).unwrap().s, "abc");
    }

    #[test]
    fn duplicate_unique_by_inserting() {
        let s = test_db("duplicate_unique_by_inserting").unwrap();
        let c = s.collection("test").unwrap();

        mk_index(&c).unwrap();
        fill_data(&c).unwrap();

        assert!(c.insert(&json!({ "s": "abc" })).is_err());
    }

    #[test]
    fn duplicate_unique_by_indexing() {
        let s = test_db("duplicate_unique_by_indexing").unwrap();
        let c = s.collection("test").unwrap();

        fill_data(&c).unwrap();
        
        assert!(c.insert(&json!({ "s": "abc" })).is_ok());
        assert!(mk_index(&c).is_err());
    }

    #[test]
    fn order_by_primary() {
        let s = test_db("order_by_primary").unwrap();
        let c = s.collection("test").unwrap();

        mk_index(&c).unwrap();
        fill_data(&c).unwrap();

        assert_found!(c.find(json_val!(null), json_val!("$asc")), 1, 2, 3, 4, 5, 6);
        assert_found!(c.find(json_val!(null), json_val!("$desc")), 6, 5, 4, 3, 2, 1);
    }

    #[test]
    fn order_by_unique() {
        let s = test_db("order_by_unique").unwrap();
        let c = s.collection("test").unwrap();

        mk_index(&c).unwrap();
        fill_data(&c).unwrap();

        assert_found!(c.find(json_val!(null), json_val!({ "s": "$asc" })), 3, 5, 6, 1, 2, 4);
        assert_found!(c.find(json_val!(null), json_val!({ "s": "$desc" })), 4, 2, 1, 6, 5, 3);
    }

    #[test]
    fn find_string_eq() {
        let s = test_db("find_string_eq").unwrap();
        let c = s.collection("test").unwrap();

        mk_index(&c).unwrap();
        fill_data(&c).unwrap();

        assert_found!(c.find(json_val!({ "s": { "$eq": "xyz" } }), json_val!("$asc")), 4);
        assert_found!(c.find(json_val!({ "n.a": { "$eq": "t1" } }), json_val!("$asc")), 2, 5);
        assert_found!(c.find(json_val!({ "n.a": { "$eq": "t2" } }), json_val!("$desc")), 6, 4, 2);
    }

    #[test]
    fn find_bool_eq() {
        let s = test_db("find_bool_eq").unwrap();
        let c = s.collection("test").unwrap();

        mk_index(&c).unwrap();
        fill_data(&c).unwrap();

        assert_found!(c.find(json_val!({ "b": { "$eq": true } }), json_val!("$asc")), 3, 4, 6);
        assert_found!(c.find(json_val!({ "b": { "$eq": false } }), json_val!("$asc")), 1, 2, 5);
    }

    #[test]
    fn find_int_eq() {
        let s = test_db("find_int_eq").unwrap();
        let c = s.collection("test").unwrap();

        mk_index(&c).unwrap();
        fill_data(&c).unwrap();

        assert_found!(c.find(json_val!({ "i": { "$eq": 1 } }), json_val!("$asc")), 2, 4);
        assert_found!(c.find(json_val!({ "i": { "$eq": 2 } }), json_val!("$asc")), 2, 3, 5);
        assert_found!(c.find(json_val!({ "n.i": { "$eq": 1 } }), json_val!("$asc")), 2);
        assert_found!(c.find(json_val!({ "n.i": { "$eq": 2 } }), json_val!("$desc")), 5, 3);
    }

    #[test]
    fn find_int_bw() {
        let s = test_db("find_int_eq").unwrap();
        let c = s.collection("test").unwrap();

        mk_index(&c).unwrap();
        fill_data(&c).unwrap();

        assert_found!(c.find(json_val!({ "i": { "$bw": [2, 3] } }), json_val!("$asc")), 2, 3, 5, 6);
        assert_found!(c.find(json_val!({ "n.i": { "$bw": [1, 2] } }), json_val!("$desc")), 5, 3, 2);
    }

    #[test]
    fn find_int_gt() {
        let s = test_db("find_int_gt").unwrap();
        let c = s.collection("test").unwrap();

        mk_index(&c).unwrap();
        fill_data(&c).unwrap();

        assert_found!(c.find(json_val!({ "i": { "$gt": 3 } }), json_val!("$asc")), 3, 4, 6);
        assert_found!(c.find(json_val!({ "n.i": { "$gt": 1 } }), json_val!("$desc")), 5, 4, 3);
    }

    #[test]
    fn find_int_ge() {
        let s = test_db("find_int_ge").unwrap();
        let c = s.collection("test").unwrap();

        mk_index(&c).unwrap();
        fill_data(&c).unwrap();

        assert_found!(c.find(json_val!({ "i": { "$ge": 3 } }), json_val!("$asc")), 3, 4, 6);
        assert_found!(c.find(json_val!({ "n.i": { "$ge": 1 } }), json_val!("$desc")), 5, 4, 3, 2);
    }

    #[test]
    fn find_and() {
        let s = test_db("find_and").unwrap();
        let c = s.collection("test").unwrap();

        mk_index(&c).unwrap();
        fill_data(&c).unwrap();

        assert_found!(c.find(json_val!({ "$and": [
            { "b": { "$eq": true } },
            { "i": { "$eq": 2 } }
        ] }), json_val!("$asc")), 3);
        assert_found!(c.find(json_val!({ "$and": [
            { "n.i": { "$eq": 2 } },
            { "i": { "$eq": 2 } }
        ] }), json_val!("$desc")), 5, 3);
    }

    #[test]
    fn find_or() {
        let s = test_db("find_and").unwrap();
        let c = s.collection("test").unwrap();

        mk_index(&c).unwrap();
        fill_data(&c).unwrap();

        assert_found!(c.find(json_val!({ "$or": [ { "b": { "$eq": true } }, { "i": { "$eq": 2 } } ] }), json_val!("$asc")), 2, 3, 4, 5, 6);
        assert_found!(c.find(json_val!({ "$or": [ { "n.i": { "$eq": 2 } }, { "i": { "$eq": 2 } } ] }), json_val!("$desc")), 5, 3, 2);
    }
}
