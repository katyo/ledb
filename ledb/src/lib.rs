/*!

# Lightweight embedded database

## Features

* Processing documents which implements `Serialize` and `Deserialize` traits from [serde](https://serde.rs/).
* Identifying documents using auto-incrementing integer primary keys.
* Indexing any fields of documents using unique or duplicated keys.
* Searching and ordering documents using indexed fields or primary key.
* Selecting documents using complex filters with fields comparing and logical operations.
* Updating documents using rich set of modifiers.
* Storing documents into independent storages so called collections.
* Flexible `query!` macro which helps write clear and readable queries.
* Using [LMDB](https://en.wikipedia.org/wiki/Lightning_Memory-Mapped_Database) as backend for document storage and indexing engine.

## Usage example

```rust
extern crate serde;
#[macro_use] extern crate serde_derive;
// This allows inserting JSON documents
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
    let db_path = ".test_dbs/my_temp_db";
    let _ = std::fs::remove_dir_all(&db_path);

    // Open storage
    let storage = Storage::new(&db_path).unwrap();
    
    // Get collection
    let collection = storage.collection("my-docs").unwrap();
    
    // Ensure indexes
    query!(index for collection
        title str unique,
        tag str,
        timestamp int unique,
    ).unwrap();
    
    // Insert JSON document
    let first_id = query!(insert into collection {
        "title": "First title",
        "tag": ["some tag", "other tag"],
        "timestamp": 1234567890,
    }).unwrap();
    
    // Insert typed document
    let second_id = collection.insert(&MyDoc {
        title: "Second title".into(),
        tag: vec![],
        timestamp: 1234567657,
    }).unwrap();

    // Find documents
    let found_docs = query!(
        find MyDoc in collection
        where title == "First title"
    ).unwrap().collect::<Result<Vec<_>, _>>().unwrap();
    
    // Update documents
    let n_affected = query!(
        update in collection modify title = "Other title"
        where title == "First title"
    ).unwrap();

    // Find documents with descending ordering
    let found_docs = query!(
        find MyDoc in collection order desc
    ).unwrap().collect::<Result<Vec<_>, _>>().unwrap();

    // Remove documents
    let n_affected = query!(
        remove from collection where title == "Other title"
    ).unwrap();
}
```

## Field names

Field name is a sequence of dot-separated identifiers which represents nesting of value in document.

For example, in document below:

```ignore
{
    "a": "abc",
    "b": {
        "c": 11
    },
    "d": [
      "a"
    ]
}
```

You can access fields by the next ways:

```ignore
a == "abc"
a += "def"
b.c > 10
b.c += 3
d == "a"
d[-1..] += ["b", "c"]
```

## Indexing

Index query example:

```ignore
query!(
    index for some_collection
        some_field Int unique, // unique index
        other_field.with.sub_field String,
        // ...next fields
)
```

### Index kinds

| Internal Type | JSON Type  | Description                  |
| ------------- | ---------  | -----------                  |
| Unique        | "uni"      | Each value is unique         |
| Duplicate     | "dup"      | The values can be duplicated |

Unique index guarantee that each value can be stored once, any duplicates disalowed.

The operation will fail in two cases:

1. When you try to insert new document which duplicate unique field
2. When you try to ensure unique index for field which have duplicates

Unique fields is pretty fit for sorting.

*TODO: Full-text index kind for searching*

### Key types

| Internal Type | JSON Type  | Description                   |
| ------------- | ---------  | -----------                   |
| Int           | "int"      | 64-bit signed integers        |
| Float         | "float"    | 64-bit floating point numbers |
| Bool          | "bool"     | boolean values                |
| String        | "string"   | UTF-8 strings                 |
| Binary        | "binary"   | raw binary data               |

## Filters

### Comparison operations

| Internal Repr          | JSON Repr                     | Query (where)     | Description           |
| -------------          | ---------------               | -------------     | -----------           |
| Eq(value)              | {"$eq": value}                | field == val      | General Equality      |
| In(Vec<value>)         | {"$in": [...values]}          | field in [...val] | One of                |
| Lt(value)              | {"$lt": value}                | field < val       | Less than             |
| Le(value)              | {"$le": value}                | field <= val      | Less than or equal    |
| Gt(value)              | {"$gt": value}                | field > val       | Greater than          |
| Ge(value)              | {"$ge": value}                | field >= val      | Greater than or equal |
| Bw(a, true, b, true)   | {"$bw": [a, true, b, true]}   | field in a..b     | Between including a b |
| Bw(a, false, b, false) | {"$bw": [a, false, b, false]} | field \<in> a..b  | Between excluding a b |
| Bw(a, true, b, false)  | {"$bw": [a, true, b, false]}  | field in> a..b    | Between incl a excl b |
| Bw(a, false, b, true)  | {"$bw": [a, false, b, true]}  | field <in a..b    | Between excl a incl b |
| Has                    | "$has"                        | field ?           | Has exists (not null) |

**NOTE: To be able to use particular field of document in filters you need create index for it first.**

Some examples:

```ignore
query!(@filter field == 123)
query!(@filter field.subfield != "abc")
query!(@filter field > 123)
query!(@filter field <= 456)
query!(@filter field in 123..456)   // [123 ... 456]
query!(@filter field <in> 123..456) // (123 ... 456)
query!(@filter field <in 123..456)  // (123 ... 456]
query!(@filter field in> 123..456)  // [123 ... 456)
```

### Logical operations

| Internal Repr    | JSON Repr              | Query (where)          | Description         |
| -------------    | ---------------        | -------------          | -----------         |
| Not(Box<filter>) | {"$not": filter}       | ! filter               | Filter is false     |
| And(Vec<filter>) | {"$and": [...filters]} | filter &&   ...filters | All filters is true |
| Or(Vec<filter>)  | {"$or":  [...filters]} | filter \|\| ...filters | Any filter is true  |

**NOTE: Be careful with using complex ORs and global NOTs since it may slow down your queries.**

Some examples:

```ignore
// negate filter condition
query!(@filter ! field == "abc")

// and filter conditions
query!(@filter field > 123 && field <= 456)

// or filter conditions
query!(@filter field <= 123 || field > 456)
```

## Results ordering

| Internal Repr      | JSON Repr          | Query (where)       | Description                        |
| -------------      | ---------------    | -------------       | -----------                        |
| Primary(Asc)       | "$asc"             | >, asc (default)    | Ascending ordering by primary key  |
| Primary(Desc)      | "$desc"            | <, desc             | Descending ordering by primary key |
| Field(field, Asc)  | {"field": "$asc"}  | field >, field asc  | Ascending ordering by field        |
| Field(field, Desc) | {"field": "$desc"} | field <, field desc | Descending ordering by field       |

Examples:

```ignore
// ascending ordering by primary key
query!(@order >)
query!(@order asc)

// descending ordering by primary key
query!(@order <)
query!(@order desc)

// ascending ordering by field
query!(@order by field >)
query!(@order by field asc)

// descending ordering by other.field
query!(@order by other.field <)
query!(@order by other.field desc)
```

## Modifiers

| Internal Repr              | JSON Repr                       | Query (where)         | Description                |
| -------------              | ---------------                 | -------------         | -----------                |
| Set(value)                 | {"$set": value}                 | field = value         | Set field value            |
| Delete                     | "$delete"                       | field ~               | Delete field               |
| Add(value)                 | {"$add": value}                 | field += value        | Add value to field         |
| Sub(value)                 | {"$sub": value}                 | field -= value        | Substract value from field |
| Mul(value)                 | {"$mul": value}                 | field *= value        | Multiply field to value    |
| Div(value)                 | {"$div": value}                 | field /= value        | Divide field to value      |
| Toggle                     | "$toggle"                       | field !               | Toggle boolean field       |
| Replace(pat, sub)          | {"$replace": ["pat", "sub"]}    | field ~= "pat" "sub"  | Replace using regexp       |
| Splice(from, to, Vec<ins>) | {"$splice": [from, to]}         | field[from..to] ~     | Remove from an array       |
| Splice(from, to, Vec<ins>) | {"$splice": [from, to, ...ins]} | field[from..to] = ins | Splice an array            |
| Merge(object)              | {"$merge": object}              | field ~= object       | Merge an object            |

The negative range value means position from end of an array:

* -1 the end of an array
* -2 the last element
* -3 the element before the last
* ...and so on

## Extended behavior of modifiers

| Internal Repr | JSON Repr             | Query (where)       | Description                               |
| ------------- | ---------------       | -------------       | -----------                               |
| Add(values)   | {"$add": [...values]} | field += [..values] | Add unique values to an array as a set    |
| Sub(values)   | {"$sub": [...values]} | field -= [..values] | Remove unique values to an array as a set |
| Add(text)     | {"$add": "text"}      | field += "text"     | Append text to a string                   |

Examples:

```ignore
// set single fields
query!(@modify field = 123)
query!(@modify other.field = "abc")

// set multiple fields
query!(@modify 
    field = 1;
    other.field = "abc";
)

// numeric operations
query!(@modify field += 1) // add value to field
query!(@modify field -= 1) // substract value from field
query!(@modify field *= 1) // multiply field to value
query!(@modify field /= 1) // divide field to value

query!(@modify - field) // remove field
query!(@modify ! field) // toggle boolean field

query!(@modify str += "addon") // append piece to string
query!(@modify str ~= "abc" "def") // regexp replace

// modify array as list
query!(@modify list[0..0] = [1, 2, 3]) // prepend to array
query!(@modify list[-1..0] = [1, 2, 3]) // append to array
query!(@modify - list[1..2]) // remove from array
query!(@modify list[1..2] = [1, 2, 3]) // splice array

// modify array as set
query!(@modify set += [1, 2, 3]) // add elements
query!(@modify set -= [4, 5, 6]) // remove elements

// merge an object
query!(@modify obj ~= { a: true, b: "abc", c: 123 })
query!(@modify obj ~= extra)
```

*/

extern crate byteorder;
extern crate ordered_float;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate lmdb_zero as lmdb;
extern crate regex;
extern crate ron;
extern crate serde_cbor;
extern crate supercow;

#[cfg(test)]
#[macro_use]
extern crate serde_json;

extern crate dirs;
extern crate dunce;

#[cfg(test)]
#[macro_use]
mod test;

mod collection;
mod document;
mod enumerate;
mod error;
mod filter;
mod float;
mod index;
mod modify;
mod pool;
mod selection;
mod storage;
mod value;

#[macro_use]
mod macros;

pub use collection::{Collection, DocumentsIterator};
pub use document::{to_value, Document, Identifier, Primary, Value};
pub use error::{Error, Result, ResultWrap};
pub use filter::{Comp, Cond, Filter, Order, OrderKind};
pub use index::IndexKind;
pub use modify::{Action, Modify, WrappedRegex};
pub use storage::{Info, Stats, Storage};
pub use value::{KeyData, KeyType};

use collection::CollectionDef;
use enumerate::{Enumerable, Serial, SerialGenerator};
use index::{Index, IndexDef};
use pool::Pool;
use selection::Selection;
use storage::{DatabaseDef, StorageData};

#[cfg(test)]
mod tests {
    use super::{Collection, Document, Primary, Result, Value};
    use test::test_db;

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
        c.insert(
            &json!({ "s": "def", "b": false, "i": [1, 2], "n": { "i": 1, "a": ["t1", "t2"] } }),
        )?;
        c.insert(&json!({ "s": "123", "b": true, "i": [2, 3, 4], "n": { "i": 2, "a": [] } }))?;
        c.insert(
            &json!({ "s": "xyz", "b": true, "i": [1, 5, 4], "n": { "i": 3, "a": ["t2", "t4"] } }),
        )?;
        c.insert(&json!({ "s": "321", "b": false, "i": [2], "n": { "i": 2, "a": ["t4", "t1"] } }))?;
        c.insert(&json!({ "s": "456", "b": true, "i": [3, 5], "n": { "i": -11, "a": ["t2"] } }))?;
        Ok(())
    }

    fn mk_index(c: &Collection) -> Result<()> {
        query!(index for c
               s str unique,
               b bool,
               i integer,
               f float,
               n.i int,
               n.a string)
    }

    #[test]
    fn insert_documents() {
        let s = test_db("insert").unwrap();
        let c = s.collection("test").unwrap();

        assert_eq!(query!(insert into c &Doc::default()).unwrap(), 1);
        assert_eq!(query!(insert into c &Doc::default()).unwrap(), 2);
        assert_eq!(
            query!(insert into c {
            "s": "",
            "b": false,
            "i": 0,
            "f": 0.0,
            "n": {
                "i": 0,
                "a": []
            }
        }).unwrap(),
            3
        );
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

        assert_eq!(
            &query!(find Doc in c where s == "abc")
                .unwrap()
                .next()
                .unwrap()
                .unwrap()
                .s,
            "abc"
        );
    }

    #[test]
    fn index_after_insert() {
        let s = test_db("index_after_insert").unwrap();
        let c = s.collection("test").unwrap();

        fill_data(&c).unwrap();
        mk_index(&c).unwrap();

        assert_eq!(
            &query!(find Doc in c where s == "abc")
                .unwrap()
                .next()
                .unwrap()
                .unwrap()
                .s,
            "abc"
        );
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
        assert!(query!(insert into c { "s": "abc" }).is_ok());
        assert!(mk_index(&c).is_err());
    }

    #[test]
    fn order_by_primary() {
        let s = test_db("order_by_primary").unwrap();
        let c = s.collection("test").unwrap();

        mk_index(&c).unwrap();
        fill_data(&c).unwrap();

        assert_found!(query!(find in c order >), 1, 2, 3, 4, 5, 6);
        assert_found!(query!(find in c order <), 6, 5, 4, 3, 2, 1);
    }

    #[test]
    fn order_by_unique() {
        let s = test_db("order_by_unique").unwrap();
        let c = s.collection("test").unwrap();

        mk_index(&c).unwrap();
        fill_data(&c).unwrap();

        assert_eq!(
            query!(find Value in c order by s >).unwrap().size_hint(),
            (6, Some(6))
        );
        assert_found!(query!(find in c order by s >), 3, 5, 6, 1, 2, 4);
        assert_found!(query!(find in c order by s <), 4, 2, 1, 6, 5, 3);
    }

    #[test]
    fn find_string_eq() {
        let s = test_db("find_string_eq").unwrap();
        let c = s.collection("test").unwrap();

        mk_index(&c).unwrap();
        fill_data(&c).unwrap();

        assert_eq!(
            query!(find Value in c where s == "xyz")
                .unwrap()
                .size_hint(),
            (1, Some(1))
        );
        assert_found!(query!(find in c where s == "xyz" order >), 4);
        assert_found!(query!(find in c where n.a == "t1"), 2, 5);
        assert_eq!(
            query!(find Value in c where n.a == "t2" order <)
                .unwrap()
                .size_hint(),
            (3, Some(3))
        );
        assert_found!(query!(find in c where n.a == "t2" order desc), 6, 4, 2);
    }

    #[test]
    fn find_bool_eq() {
        let s = test_db("find_bool_eq").unwrap();
        let c = s.collection("test").unwrap();

        mk_index(&c).unwrap();
        fill_data(&c).unwrap();

        assert_found!(query!(find in c where b == true), 3, 4, 6);
        assert_found!(query!(find in c where b == false), 1, 2, 5);
    }

    #[test]
    fn find_int_eq() {
        let s = test_db("find_int_eq").unwrap();
        let c = s.collection("test").unwrap();

        mk_index(&c).unwrap();
        fill_data(&c).unwrap();

        assert_found!(query!(find in c where i == 1), 2, 4);
        assert_found!(query!(find in c where i == 2), 2, 3, 5);
        assert_found!(query!(find in c where n.i == 1), 2);
        assert_found!(query!(find in c where n.i == 2 order <), 5, 3);
    }

    #[test]
    fn find_string_in() {
        let s = test_db("find_string_in").unwrap();
        let c = s.collection("test").unwrap();

        mk_index(&c).unwrap();
        fill_data(&c).unwrap();

        assert_eq!(
            query!(find Value in c where s in ["abc", "xyz"])
                .unwrap()
                .size_hint(),
            (2, Some(2))
        );
        assert_found!(query!(find in c where s in ["abc", "xyz"] order >), 1, 4);
        assert_found!(query!(find in c where n.a in ["t1", "t4"] order >), 2, 4, 5);
        assert_eq!(
            query!(find Value in c where n.a in ["t2"] order <)
                .unwrap()
                .size_hint(),
            (3, Some(3))
        );
        assert_found!(query!(find in c where n.a in ["t2"] order desc), 6, 4, 2);
    }

    #[test]
    fn find_int_in() {
        let s = test_db("find_int_in").unwrap();
        let c = s.collection("test").unwrap();

        mk_index(&c).unwrap();
        fill_data(&c).unwrap();

        assert_found!(query!(find in c where i in [1, 5]), 2, 4, 6);
        assert_found!(query!(find in c where i in [2]), 2, 3, 5);
        assert_found!(query!(find in c where n.i in [1, 3]), 2, 4);
        assert_found!(query!(find in c where n.i in [2] order <), 5, 3);
    }

    #[test]
    fn find_int_bw() {
        let s = test_db("find_int_bw").unwrap();
        let c = s.collection("test").unwrap();

        mk_index(&c).unwrap();
        fill_data(&c).unwrap();

        assert_found!(query!(find in c where i in 2..3), 2, 3, 5, 6);
        assert_found!(query!(find in c where n.i in 1..2 order desc), 5, 3, 2);
    }

    #[test]
    fn find_int_gt() {
        let s = test_db("find_int_gt").unwrap();
        let c = s.collection("test").unwrap();

        mk_index(&c).unwrap();
        fill_data(&c).unwrap();

        assert_found!(query!(find in c where i > 3), 3, 4, 6);
        assert_found!(query!(find in c where n.i > 1 order <), 5, 4, 3);
    }

    #[test]
    fn find_int_ge() {
        let s = test_db("find_int_ge").unwrap();
        let c = s.collection("test").unwrap();

        mk_index(&c).unwrap();
        fill_data(&c).unwrap();

        assert_found!(query!(find in c where i >= 3), 3, 4, 6);
        assert_found!(query!(find in c where n.i >= 1 order desc), 5, 4, 3, 2);
    }

    #[test]
    fn find_int_lt() {
        let s = test_db("find_int_lt").unwrap();
        let c = s.collection("test").unwrap();

        mk_index(&c).unwrap();
        fill_data(&c).unwrap();

        assert_found!(query!(find in c where i < 3), 2, 3, 4, 5);
        assert_found!(query!(find in c where n.i < 2 order desc), 6, 2);
    }

    #[test]
    fn find_int_le() {
        let s = test_db("find_int_le").unwrap();
        let c = s.collection("test").unwrap();

        mk_index(&c).unwrap();
        fill_data(&c).unwrap();

        assert_found!(query!(find in c where i <= 3), 2, 3, 4, 5, 6);
        assert_found!(query!(find in c where n.i <= 2 order desc), 6, 5, 3, 2);
    }

    #[test]
    fn find_has() {
        let s = test_db("find_has").unwrap();
        let c = s.collection("test").unwrap();

        mk_index(&c).unwrap();
        fill_data(&c).unwrap();

        assert_found!(query!(find in c where i?), 2, 3, 4, 5, 6);
        assert_found!(query!(find in c where n.i? order desc), 6, 5, 4, 3, 2);
    }

    #[test]
    fn find_and() {
        let s = test_db("find_and").unwrap();
        let c = s.collection("test").unwrap();

        mk_index(&c).unwrap();
        fill_data(&c).unwrap();

        assert_found!(query!(find in c where b == true && i == 2), 3);
        assert_found!(query!(find in c where n.i == 2 && i == 2 order <), 5, 3);
    }

    #[test]
    fn find_or() {
        let s = test_db("find_or").unwrap();
        let c = s.collection("test").unwrap();

        mk_index(&c).unwrap();
        fill_data(&c).unwrap();

        assert_found!(query!(find in c where b == true || i == 2), 2, 3, 4, 5, 6);
        assert_found!(query!(find in c where n.i == 2 || i == 2 order <), 5, 3, 2);
    }

    #[test]
    fn remove_eq_str() {
        let s = test_db("remove_eq_str").unwrap();
        let c = s.collection("test").unwrap();

        mk_index(&c).unwrap();
        fill_data(&c).unwrap();

        assert_found!(query!(find in c where s == "def"), 2);
        assert_eq!(query!(remove from c where s == "def").unwrap(), 1);
        assert_found!(query!(find in c where s == "def"));
        assert_found!(query!(find in c where s == "abc"), 1);
    }

    #[test]
    fn remove_ge_int() {
        let s = test_db("remove_ge_int").unwrap();
        let c = s.collection("test").unwrap();

        mk_index(&c).unwrap();
        fill_data(&c).unwrap();

        assert_found!(query!(find in c where i >= 4), 3, 4, 6);
        assert_eq!(query!(remove from c where i >= 4).unwrap(), 3);
        assert_found!(query!(find in c where i >= 4));
        assert_found!(query!(find in c where i >= 2), 2, 5);
    }

    #[test]
    fn update_set_eq_str() {
        let s = test_db("update_set_eq_str").unwrap();
        let c = s.collection("test").unwrap();

        mk_index(&c).unwrap();
        fill_data(&c).unwrap();

        assert_found!(query!(find in c where s == "def"), 2);
        assert_eq!(
            query!(update in c modify s = "klm" where s == "def").unwrap(),
            1
        );
        assert_found!(query!(find in c where s == "def"));
        assert_found!(query!(find in c where s == "abc"), 1);
        assert_found!(query!(find in c where s == "klm"), 2);
    }
}
