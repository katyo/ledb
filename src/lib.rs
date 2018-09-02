extern crate bytes;
extern crate serde;
#[macro_use] extern crate serde_derive;
extern crate serde_cbor;
extern crate serde_json;
extern crate lmdb_zero as lmdb;
extern crate liblmdb_sys as ffi;

mod key;
mod val;

use std::fmt::Display;
use std::fs::create_dir_all;
use std::sync::Arc;
use std::collections::HashSet;
use lmdb::{EnvBuilder, Environment, open::Flags as OpenFlags, error::Error as LmdbError, Database, DatabaseOptions, ReadTransaction, Cursor, CursorIter, MaybeOwned};

pub const NOT_FOUND: LmdbError = LmdbError::Code(ffi::MDB_NOTFOUND);

pub struct Storage<'db> {
    env: Arc<Environment>,
    db: Arc<Database<'db>>,
    coll: Vec<Collection<'db>>,
}

impl<'db> Storage<'db> {
    pub fn open<P: AsRef<str>>(path: P) -> Result<Self, String> {
        let path = path.as_ref();
        let mut bld = EnvBuilder::new().map_err(display_to_string)?;
        bld.set_maxdbs(1023).map_err(display_to_string)?;

        create_dir_all(path).map_err(display_to_string)?;
        let env = Arc::new(unsafe { bld.open(path, OpenFlags::empty(), 0o600) }
        .map_err(display_to_string)?);

        let db = Arc::new(Database::open(
            env.clone(), None, &DatabaseOptions::defaults())
            .map_err(display_to_string)?);

        {
            let db = Database::open(
                env.clone(), Some("user"), &DatabaseOptions::create_map::<[u8;8]>())
                .map_err(display_to_string)?;

            let db = Database::open(
                env.clone(), Some("user.name.u"), &DatabaseOptions::create_map::<str>())
                .map_err(display_to_string)?;

            let db = Database::open(
                env.clone(), Some("user.status.d"), &DatabaseOptions::create_multimap::<str, [u8;8]>())
                .map_err(display_to_string)?;
        }

        let coll = list_collections(env.clone(), db.clone())?;
        
        Ok(Self { env: env.clone(), db: db.clone(), coll })
    }

    /*
    pub fn collection<N: AsRef<str>>(name: N) -> &Collection {
        
    }*/
}

#[derive(Debug)]
pub struct Collection<'db> {
    pub name: String,
    pub indexes: Vec<Index<'db>>,
    pub db: Arc<Database<'db>>,
}

fn list_collections<'db>(env: Arc<Environment>, db: Arc<Database<'static>>) -> Result<Vec<Collection<'db>>, String> {
    {
        let txn = ReadTransaction::new(env.clone()).map_err(display_to_string)?;
        let lst = {
            let cursor = txn.cursor(db.clone()).map_err(display_to_string)?;
            let access = txn.access();
            
            CursorIter::new(MaybeOwned::Owned(cursor), &access,
                            |c, a| c.first(a), Cursor::next::<str,[u8]>)
                .map_err(display_to_string)?
                .map(|res| res.map(|(key, _val)| key.split('.').next().unwrap()))
                .collect::<Result<HashSet<_>, _>>()
                .map_err(display_to_string)?
                .iter().map(|s| (*s).into())
                   .collect::<Vec<String>>()
        };
        lst
    }.iter().cloned().map(move |name| {
        let indexes = list_indexes(env.clone(), db.clone(), &name)?;
        let coll_db = Database::open(
            env.clone(), Some(&name), &DatabaseOptions::create_map::<[u8;8]>())
            .map_err(display_to_string)?;
        Ok(Collection { name, indexes, db: Arc::new(coll_db) })
    }).collect::<Result<Vec<_>, _>>()
}

#[derive(Debug)]
pub enum IndexKind {
    Unique,
    Duplicate,
}

#[derive(Debug)]
pub struct Index<'db> {
    pub path: Vec<String>,
    pub kind: IndexKind,
    pub db: Arc<Database<'db>>,
}

fn list_indexes<'db, S: AsRef<str>>(env: Arc<Environment>, db: Arc<Database<'static>>, coll: S) -> Result<Vec<Index<'db>>, String> {
    let coll = coll.as_ref();

    let txn = ReadTransaction::new(env.clone()).map_err(display_to_string)?;

    {
        let mut cursor = txn.cursor(db).map_err(display_to_string)?;
        let access = txn.access();

        let (fst, _): (&str, &[u8]) = cursor.seek_range_k(&access, coll)
            .map_err(display_to_string)?;

        if fst != coll {
            return Err("Invalid collection".into());
        }

        let mut indexes = Vec::new();

        loop {
            let (db_name, mut key) = match cursor.next::<str, [u8]>(&access) {
                Ok((key,_)) => (key, key.split('.')),
                Err(NOT_FOUND) => break,
                Err(e) => return Err(display_to_string(e)),
            };

            if key.next() != Some(coll) {
                break
            }

            let mut path: Vec<String> = key.map(String::from).collect();
            
            if path.len() < 2 {
                continue;
            }

            let kind = match path.pop().unwrap().as_str() {
                "u" => IndexKind::Unique,
                "d" => IndexKind::Duplicate,
                _ => continue,
            };

            let index_db = Database::open(
                env.clone(), Some(db_name), &match kind {
                    IndexKind::Unique => DatabaseOptions::create_map::<str>(),
                    IndexKind::Duplicate => DatabaseOptions::create_multimap::<str, [u8;8]>(),
                })
                .map_err(display_to_string)?;

            indexes.push(Index { path, kind, db: Arc::new(index_db) });
        }

        Ok(indexes)
    }
}

fn display_to_string<T: Display>(e: T) -> String {
    format!("{}", e)
}

#[cfg(test)]
mod test {
    use std::fs::remove_dir_all;
    use super::{Storage};

    const DB_DIR: &'static str = ".test_db";

    #[test]
    fn test_open() {
        let s = Storage::open(DB_DIR).unwrap();

        assert!(false);

        remove_dir_all(DB_DIR).unwrap();
    }
}
