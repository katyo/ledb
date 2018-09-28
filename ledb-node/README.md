# LEDB interface for NodeJS

[![License: MIT](https://img.shields.io/badge/License-MIT-brightgreen.svg)](https://opensource.org/licenses/MIT)
[![npm version](https://badge.fury.io/js/ledb.svg)](https://badge.fury.io/js/ledb)
[![npm downloads](https://img.shields.io/npm/dm/ledb.svg)](https://www.npmjs.com/package/ledb)
[![Travis-CI Build Status](https://travis-ci.org/katyo/ledb.svg?branch=master)](https://travis-ci.org/katyo/ledb)
[![Appveyor Build status](https://ci.appveyor.com/api/projects/status/1wrmhivii22emfxg)](https://ci.appveyor.com/project/katyo/ledb)

The **LEDB** is an attempt to implement simple but efficient, lightweight but powerful document storage.

The abbreviation *LEDB* may be treated as an Lightweight Embedded DB, also Low End DB, also Literium Engine DB, also LitE DB, and so on.

## Links

* [ledb NodeJS package on npmjs.com](https://www.npmjs.com/package/ledb)
* [ledb Rust Crate on crates.io](https://crates.io/crates/ledb)
* [ledb Rust API Docs on docs.rs](https://docs.rs/ledb)

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

```typescript
import { Storage } from 'ledb';

// Open storage
const storage = new Storage("test_db/storage");
// It allows open storage with same path multiple times

// Get storage info
console.log("Storage info:", storage.get_info());
console.log("Storage stats:", storage.get_stats());

// Get collection handle
const posts = storage.collection("post");

// Insert document
let doc_id = posts.insert({title: "Foo", tag: ["Bar", "Baz"], timestamp: 1234567890);

// Get document by id
let doc = posts.get(doc_id);
console.log("Inserted document: ", doc);

// Put new version of document
posts.put(doc);

// Delete document by id
posts.delete(doc_id);

// Ensure indexes
posts.ensure_index("title", "uni", "string")
posts.ensure_index("tag", "dup", "string")

// Get indexes
console.log("Indexes of post:", posts.get_indexes())

// Find all documents
let docs = posts.find(null);

// Find all documents with descending ordering
let docs = posts.find(null, "$desc");

// Find all documents with ascending ordering using field
let docs = posts.find(null, { timestamp: "$asc" });

// Find documents using filter
let docs = posts.find({ title: { $eq:"Foo" } });
let docs = posts.find({ $not: { title: { $eq: "Foo" } } });
let docs = posts.find({ $and: [ { timestamp: { $gt: 123456789 } } ,
                                { tag: { $eq: "Bar" } } ] },
                      { timestamp: "$desc" });
let docs = posts.find({ $or: [ { title: { $eq: "Foo" } } ,
                               { title: { $eq: "Bar" } } ] });

// Number of found documents
console.log("Found docs:", docs.count())

// Get documents one by one
for (let doc; doc = docs.next(); ) {
    console.log("Found doc:", doc);
}

// Skip N documents
docs.skip(3);

// Take N documents only
docs.take(5);

// Get all documents as an array
console.log("Found documents:", docs.collect());

// Update all documents
posts.update(null, { timestamp: { $set: 0 } });

// Update documents using filter
posts.update({ timestamp: { $le: 123456789 } }, { timestamp: { $set: 0 } });

// Remove all documents
posts.remove(null);

// Remove documents using filter
posts.remove({ timestamp: { $le: 123456789 } });
```
