extern crate byteorder;
extern crate ordered_float;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate serde_cbor;
extern crate ron;
extern crate lmdb_zero as lmdb;
extern crate regex;

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
mod modify;
mod storage;
mod collection;
mod index;

#[macro_use]
mod macros;

pub use error::{Error, Result, ResultWrap};
pub use document::{Identifier, Primary, Document, Value, to_value};
pub use storage::{Storage};
pub use collection::{Collection, DocumentsIterator};
pub use index::{IndexKind};
pub use value::{KeyType, KeyData};
pub use filter::{Filter, Comp, Cond, OrderKind, Order};
pub use modify::{Modify, Action, WrappedRegex};

use storage::{DatabaseDef};
use collection::{CollectionDef};
use index::{Index, IndexDef};
use selection::{Selection};

#[cfg(test)]
mod tests {
    use test::test_db;
    use super::{Value, Collection, Primary, Document, Result};

    macro_rules! assert_found {
        ($res:expr $(,$exp:expr)*) => {
            let ids: Vec<Primary> = vec![$($exp),*];
            assert_eq!($res.unwrap().map(|doc: Result<Document<Value>>| doc.unwrap().req_id().unwrap()).collect::<Vec<_>>(), ids)
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
        query!(ensure index c
               s String unique
               b Bool
               i Int
               f Float
               n.i Int
               n.a String)
    }

    #[test]
    fn insert_documents() {
        let s = test_db("insert").unwrap();
        let c = s.collection("test").unwrap();

        assert_eq!(query!(insert into c &Doc::default()).unwrap(), 1);
        assert_eq!(query!(insert c &Doc::default()).unwrap(), 2);
        assert_eq!(query!(insert c {
            "s": "",
            "b": false,
            "i": 0,
            "f": 0.0,
            "n": {
                "i": 0,
                "a": []
            }
        }).unwrap(), 3);
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

        assert_eq!(&query!(select Doc from c where s == "abc").unwrap().next().unwrap().unwrap().s, "abc");
    }

    #[test]
    fn index_after_insert() {
        let s = test_db("index_after_insert").unwrap();
        let c = s.collection("test").unwrap();

        fill_data(&c).unwrap();
        mk_index(&c).unwrap();
        
        assert_eq!(&query!(select Doc from c where s == "abc").unwrap().next().unwrap().unwrap().s, "abc");
    }

    #[test]
    fn duplicate_unique_by_inserting() {
        let s = test_db("duplicate_unique_by_inserting").unwrap();
        let c = s.collection("test").unwrap();

        assert!(mk_index(&c).is_ok());
        assert!(fill_data(&c).is_ok());
        assert!(query!(insert into c { "s": "abc" }).is_err());
    }

    #[test]
    fn duplicate_unique_by_indexing() {
        let s = test_db("duplicate_unique_by_indexing").unwrap();
        let c = s.collection("test").unwrap();

        assert!(fill_data(&c).is_ok());
        assert!(query!(insert c { "s": "abc" }).is_ok());
        assert!(mk_index(&c).is_err());
    }

    #[test]
    fn order_by_primary() {
        let s = test_db("order_by_primary").unwrap();
        let c = s.collection("test").unwrap();

        mk_index(&c).unwrap();
        fill_data(&c).unwrap();

        assert_found!(query!(find in c order >), 1, 2, 3, 4, 5, 6);
        assert_found!(query!(find c order by <), 6, 5, 4, 3, 2, 1);
    }

    #[test]
    fn order_by_unique() {
        let s = test_db("order_by_unique").unwrap();
        let c = s.collection("test").unwrap();

        mk_index(&c).unwrap();
        fill_data(&c).unwrap();

        assert_eq!(query!(find Value in c order by s >).unwrap().size_hint(), (6, Some(6)));
        assert_found!(query!(find c order s v), 3, 5, 6, 1, 2, 4);
        assert_found!(query!(find in c order by s ^), 4, 2, 1, 6, 5, 3);
    }

    #[test]
    fn find_string_eq() {
        let s = test_db("find_string_eq").unwrap();
        let c = s.collection("test").unwrap();

        mk_index(&c).unwrap();
        fill_data(&c).unwrap();

        assert_eq!(query!(find Value in c where s == "xyz").unwrap().size_hint(), (1, Some(1)));
        assert_found!(query!(find c where [s == "xyz"] order >), 4);
        assert_found!(query!(find in c where (n.a == "t1") order by >), 2, 5);
        assert_eq!(query!(find Value in c where [n.a == "t2"] order <).unwrap().size_hint(), (3, Some(3)));
        assert_found!(query!(find c where (n.a == "t2") order ^), 6, 4, 2);
    }

    #[test]
    fn find_bool_eq() {
        let s = test_db("find_bool_eq").unwrap();
        let c = s.collection("test").unwrap();

        mk_index(&c).unwrap();
        fill_data(&c).unwrap();

        assert_found!(query!(find c where b == true), 3, 4, 6);
        assert_found!(query!(find c where b == false), 1, 2, 5);
    }

    #[test]
    fn find_int_eq() {
        let s = test_db("find_int_eq").unwrap();
        let c = s.collection("test").unwrap();

        mk_index(&c).unwrap();
        fill_data(&c).unwrap();

        assert_found!(query!(select from c where i == 1), 2, 4);
        assert_found!(query!(select c where i == 2), 2, 3, 5);
        assert_found!(query!(find in c where n.i == 1), 2);
        assert_found!(query!(find c where [ n.i == 2 ] order <), 5, 3);
    }

    #[test]
    fn find_int_bw() {
        let s = test_db("find_int_bw").unwrap();
        let c = s.collection("test").unwrap();

        mk_index(&c).unwrap();
        fill_data(&c).unwrap();

        assert_found!(query!(find c where i in 2..3), 2, 3, 5, 6);
        assert_found!(query!(find in c where (n.i in 1..2) order ^), 5, 3, 2);
    }

    #[test]
    fn find_int_gt() {
        let s = test_db("find_int_gt").unwrap();
        let c = s.collection("test").unwrap();

        mk_index(&c).unwrap();
        fill_data(&c).unwrap();

        assert_found!(query!(find c where i > 3), 3, 4, 6);
        assert_found!(query!(find c where (n.i > 1) order <), 5, 4, 3);
    }

    #[test]
    fn find_int_ge() {
        let s = test_db("find_int_ge").unwrap();
        let c = s.collection("test").unwrap();

        mk_index(&c).unwrap();
        fill_data(&c).unwrap();

        assert_found!(query!(find in c where i >= 3), 3, 4, 6);
        assert_found!(query!(find in c where (n.i >= 1) order ^), 5, 4, 3, 2);
    }

    #[test]
    fn find_int_lt() {
        let s = test_db("find_int_lt").unwrap();
        let c = s.collection("test").unwrap();

        mk_index(&c).unwrap();
        fill_data(&c).unwrap();

        assert_found!(query!(find c where i < 3), 2, 3, 4, 5);
        assert_found!(query!(find c where [n.i < 2] order ^), 6, 2);
    }

    #[test]
    fn find_int_le() {
        let s = test_db("find_int_le").unwrap();
        let c = s.collection("test").unwrap();

        mk_index(&c).unwrap();
        fill_data(&c).unwrap();

        assert_found!(query!(find c where i <= 3), 2, 3, 4, 5, 6);
        assert_found!(query!(find c where [n.i <= 2] order ^), 6, 5, 3, 2);
    }

    #[test]
    fn find_and() {
        let s = test_db("find_and").unwrap();
        let c = s.collection("test").unwrap();

        mk_index(&c).unwrap();
        fill_data(&c).unwrap();

        assert_found!(query!(find in c where b == true && i == 2), 3);
        assert_found!(query!(find c where (n.i == 2 && i == 2) order <), 5, 3);
    }

    #[test]
    fn find_or() {
        let s = test_db("find_or").unwrap();
        let c = s.collection("test").unwrap();

        mk_index(&c).unwrap();
        fill_data(&c).unwrap();

        assert_found!(query!(find c where b == true || i == 2), 2, 3, 4, 5, 6);
        assert_found!(query!(find c where [n.i == 2 || i == 2] order by <), 5, 3, 2);
    }

    #[test]
    fn remove_eq_str() {
        let s = test_db("remove_eq_str").unwrap();
        let c = s.collection("test").unwrap();

        mk_index(&c).unwrap();
        fill_data(&c).unwrap();

        assert_found!(query!(find c where s == "def"), 2);
        assert_eq!(query!(delete from c where s == "def").unwrap(), 1);
        assert_found!(query!(find c where s == "def"));
        assert_found!(query!(find c where s == "abc"), 1);
    }

    #[test]
    fn remove_ge_int() {
        let s = test_db("remove_ge_int").unwrap();
        let c = s.collection("test").unwrap();

        mk_index(&c).unwrap();
        fill_data(&c).unwrap();

        assert_found!(query!(find in c where i >= 4), 3, 4, 6);
        assert_eq!(query!(remove c where i >= 4).unwrap(), 3);
        assert_found!(query!(find c where i >= 4));
        assert_found!(query!(find c where i >= 2), 2, 5);
    }

    #[test]
    fn update_set_eq_str() {
        let s = test_db("update_set_eq_str").unwrap();
        let c = s.collection("test").unwrap();

        mk_index(&c).unwrap();
        fill_data(&c).unwrap();

        assert_found!(query!(find c where s == "def"), 2);
        assert_eq!(query!(update c [ s = "klm" ] where s == "def").unwrap(), 1);
        assert_found!(query!(find c where s == "def"));
        assert_found!(query!(find c where s == "abc"), 1);
        assert_found!(query!(find c where s == "klm"), 2);
    }
}
