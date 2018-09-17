/*!

# Lightweight embedded database

## Features

* Processing documents which implements `Serialize` and `Deserialize` traits from [serde](https://serde.rs/).
* Identifying documents using auto-incrementing integer primary keys.
* Indexing any fields of documents using unique or duplicated keys.
* Searching and ordering documents using indexed fields or primary key.
* Updating documents using rich set of modifiers.
* Storing documents into independent storages so called collections.
* Using [LMDB](https://en.wikipedia.org/wiki/Lightning_Memory-Mapped_Database) as backend for document storage and indexing engine.

## Supported key types

| Internal Type | Serde Type | Description                   |
| ------------- | ---------  | -----------                   |
| `Int`         | `"int"`    | 64-bit signed integers        |
| `Float`       | `"float"`  | 64-bit floating point numbers |
| `Bool`        | `"bool"`   | boolean values                |
| `String`      | `"string"` | UTF-8 strings                 |
| `Binary`      | `"binary"` | raw binary data               |

## Supported filters

### Comparison operations

| Internal Repr            | Serde/JSON Repr                 | Query (where)       | Description           |
| -------------            | ---------------                 | -------------       | -----------           |
| `Eq(value)`              | `{"$eq": value}`                | `field == val`      | General Equality      |
| `In(Vec<value>)`         | `{"$in": [...values]}`          | `field in [...val]` | One of                |
| `Lt(value)`              | `{"$lt": value}`                | `field < val`       | Less than             |
| `Le(value)`              | `{"$le": value}`                | `field <= val`      | Less than or equal    |
| `Gt(value)`              | `{"$gt": value}`                | `field > val`       | Greater than          |
| `Ge(value)`              | `{"$ge": value}`                | `field >= val`      | Greater than or equal |
| `Bw(a, true, b, true)`   | `{"$bw": [a, true, b, true]}`   | `field in a..b`     | Between including a b |
| `Bw(a, false, b, false)` | `{"$bw": [a, false, b, false]}` | `field <in> a..b`   | Between excluding a b |
| `Bw(a, true, b, false)`  | `{"$bw": [a, true, b, false]}`  | `field in> a..b`    | Between incl a excl b |
| `Bw(a, false, b, true)`  | `{"$bw": [a, false, b, true]}`  | `field <in a..b`    | Between excl a incl b |
| `Has`                    | `"$has"`                        | `field ?`           | Has exists (not null) |

### Logical operations

| Internal Repr      | Serde/JSON Repr          | Query (where)            | Description         |
| -------------      | ---------------          | -------------            | -----------         |
| `Not(Box<filter>)` | `{"$not": filter}`       | `! filter`               | Filter is false     |
| `And(Vec<filter>)` | `{"$and": [...filters]}` | `filter && ...filters`   | All filters is true |
| `Or(Vec<filter>)`  | `{"$or": [...filters]}`  | `filter \|\| ...filters` | Any filter is true  |

## Supported ordering

| Internal Repr        | Serde/JSON Repr      | Query (where)        | Description                        |
| -------------        | ---------------      | -------------        | -----------                        |
| `Primary(Asc)`       | `"$asc"`             | `>`, `v` (default)   | Ascending ordering by primary key  |
| `Primary(Desc)`      | `"$desc"`            | `<`, `^`             | Descending ordering by primary key |
| `Field(field, Asc)`  | `{"field": "$asc"}`  | `field >`, `field v` | Ascending ordering by field        |
| `Field(field, Desc)` | `{"field": "$desc"}` | `field <`, `field ^` | Descending ordering by field       |

## Supported modifiers

| Internal Repr                | Serde/JSON Repr                   | Query (where)           | Description                |
| -------------                | ---------------                   | -------------           | -----------                |
| `Set(value)`                 | `{"$set": value}`                 | `field = value`         | Set field value            |
| `Delete`                     | `"$delete"`                       | `- field`               | Delete field               |
| `Add(value)`                 | `{"$add": value}`                 | `field += value`        | Add value to field         |
| `Sub(value)`                 | `{"$sub": value}`                 | `field -= value`        | Substract value from field |
| `Mul(value)`                 | `{"$mul": value}`                 | `field *= value`        | Multiply field to value    |
| `Div(value)`                 | `{"$div": value}`                 | `field /= value`        | Divide field to value      |
| `Toggle`                     | `"$toggle"`                       | `! field`               | Toggle boolean field       |
| `Replace(pat, sub)`          | `{"$replace": ["pat", "sub"]}`    | `field ~= "pat" "sub"`  | Replace using regexp       |
| `Splice(off, del, Vec<ins>)` | `{"$splice": [off, del]}`         | `- field[off..del]`     | Remove from an array       |
| `Splice(off, del, Vec<ins>)` | `{"$splice": [off, del, ...ins]}` | `field[off..del] = ins` | Splice an array            |
| `Merge(object)`              | `{"$merge": object}`              | `field ~= object`       | Merge an object            |

## Extended behavior of modifiers

| Internal Repr | Serde/JSON Repr         | Query (where)         | Description                               |
| ------------- | ---------------         | -------------         | -----------                               |
| `Add(values)` | `{"$add": [...values]}` | `field += [..values]` | Add unique values to an array as a set    |
| `Sub(values)` | `{"$sub": [...values]}` | `field -= [..values]` | Remove unique values to an array as a set |
| `Add(text)`   | `{"$add": "text"}`      | `field += "text"`     | Append text to a string                   |

## Usage example

```rust
extern crate serde;
#[macro_use] extern crate serde_derive;
#[macro_use] extern crate serde_json;
#[macro_use] extern crate ledb;

use ledb::{Storage, IndexKind, KeyType, Filter, Comp, Order, OrderKind};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct MyDoc {
    title: String,
    tag: Vec<String>,
    timestamp: u32,
}

fn main() {
    let storage = Storage::open("temp-db").unwrap();
    
    let collection = storage.collection("my-docs").unwrap();
    
    query!(ensure index for collection
        title String unique
        tag String
        timestamp Int unique
    ).unwrap();
    
    let first_id = query!(insert into collection {
        "title": "First title",
        "tag": ["some tag", "other tag"],
        "timestamp": 1234567890,
    }).unwrap();
    
    let second_id = collection.insert(&MyDoc {
        title: "Second title".into(),
        tag: vec![],
        timestamp: 1234567657,
    }).unwrap();

    let found_docs = query!(
        find MyDoc in collection
        where title == "First title"
    ).unwrap().collect::<Result<Vec<_>, _>>().unwrap();
    
    let n_affected = query!(
        update collection modify [title = "Other title"]
        where title == "First title"
    ).unwrap();

    let found_docs = query!(
        find MyDoc in collection order ^
    ).unwrap().collect::<Result<Vec<_>, _>>().unwrap();

    std::fs::remove_dir_all("temp-db").unwrap();
}
```

*/

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
    fn find_string_in() {
        let s = test_db("find_string_in").unwrap();
        let c = s.collection("test").unwrap();

        mk_index(&c).unwrap();
        fill_data(&c).unwrap();

        assert_eq!(query!(find Value in c where s in ["abc", "xyz"]).unwrap().size_hint(), (2, Some(2)));
        assert_found!(query!(find c where [s in ["abc", "xyz"]] order >), 1, 4);
        assert_found!(query!(find in c where (n.a in ["t1", "t4"]) order by >), 2, 4, 5);
        assert_eq!(query!(find Value in c where [n.a in ["t2"]] order <).unwrap().size_hint(), (3, Some(3)));
        assert_found!(query!(find c where (n.a in ["t2"]) order ^), 6, 4, 2);
    }

    #[test]
    fn find_int_in() {
        let s = test_db("find_int_in").unwrap();
        let c = s.collection("test").unwrap();

        mk_index(&c).unwrap();
        fill_data(&c).unwrap();

        assert_found!(query!(select from c where i in [1, 5]), 2, 4, 6);
        assert_found!(query!(select c where i in [2]), 2, 3, 5);
        assert_found!(query!(find in c where n.i in [1, 3]), 2, 4);
        assert_found!(query!(find c where [ n.i in [2] ] order <), 5, 3);
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
    fn find_has() {
        let s = test_db("find_has").unwrap();
        let c = s.collection("test").unwrap();

        mk_index(&c).unwrap();
        fill_data(&c).unwrap();

        assert_found!(query!(find c where i?), 2, 3, 4, 5, 6);
        assert_found!(query!(find c where [n.i?] order ^), 6, 5, 4, 3, 2);
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
