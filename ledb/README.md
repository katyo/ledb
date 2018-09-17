# Lightweight embedded database

[![License: MIT](https://img.shields.io/badge/License-MIT-brightgreen.svg)](https://opensource.org/licenses/MIT)
[![Travis-CI Build Status](https://travis-ci.org/katyo/ledb.svg?branch=master)](https://travis-ci.org/katyo/ledb)
[![Appveyor Build status](https://ci.appveyor.com/api/projects/status/1wrmhivii22emfxg)](https://ci.appveyor.com/project/katyo/ledb)

The **LEDB** is an attempt to implement simple but efficient, lightweight but powerful document storage.

The abbreviation *LEDB* may be treated as an Lightweight Embedded DB, also Low End DB, also Literium Engine DB, also LitE DB, and so on.

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
| `Replace(pat, sub)`          | `{"$replace": [pat, sub]}`        | `field ~= pat sub`      | Replace using regexp       |
| `Splice(off, del, Vec<ins>)` | `{"$splice": [off, del, ...ins]}` | `field[off..del] = ins` | Splice an array            |
| `Merge(object)`              | `{"$merge": object}`              | `field ~= object`       | Merge an object            |

## Usage example

```rust
extern crate serde;
#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate ledb;

use ledb::{Storage, IndexKind, KeyType, Filter, Comp, Order, OrderKind};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct MyDoc {
    title: String,
    tag: Vec<String>,
    timestamp: u32,
}

fn main() {
    let storage = Storage::open("my-db").unwrap();
    
    let collection = storage.collection("my-docs");
    
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
    
    let second_id = collection.insert(MyDoc {
        title: "Second title".into(),
        tag: vec![],
        "timestamp": 1234567657,
    }).unwrap();
    
    let n_affected = query!(update collection modify [title = "Other title"]
                            where title == "First title")
        .unwrap();
    
    let found_docs = query!(find MyDoc in collection
                            where title == "First title")
        .unwrap().collect::<Result<Vec<_>>, _>();
}
```
