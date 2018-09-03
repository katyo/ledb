use std::sync::{Arc};
use lmdb::{Environment, put::Flags as PutFlags, Database, DatabaseOptions, ReadTransaction, WriteTransaction};

use super::{Document, Binary};
use key::{IntoKey};
use types::{document_field, ResultWrap, NOT_FOUND};
use std::iter::once;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IndexKind {
    Unique,
    Duplicate,
}

pub struct Index {
    pub(crate) path: String,
    pub(crate) kind: IndexKind,
    pub(crate) db: Arc<Database<'static>>,
}

impl Index {
    pub fn bootstrap<S: AsRef<str>>(env: Arc<Environment>, db: Arc<Database<'static>>, coll: S) -> Result<Vec<Arc<Index>>, String> {
        let coll = coll.as_ref();
        
        let txn = ReadTransaction::new(env.clone()).wrap_err()?;
        
        {
            let mut cursor = txn.cursor(db).wrap_err()?;
            let access = txn.access();
            
            let (fst, _): (&str, &[u8]) = cursor.seek_range_k(&access, coll)
                                                .wrap_err()?;
            
            if fst != coll {
                return Err("Invalid collection".into());
            }
            
            let mut indexes = Vec::new();
            
            loop {
                let (db_name, mut key) = match cursor.next::<str, [u8]>(&access) {
                    Ok((key,_)) => (key, key.split('.')),
                    Err(NOT_FOUND) => break,
                    Err(e) => return Err(e).wrap_err(),
                };
                
                if key.next() != Some(coll) {
                    break
                }
                
                let mut path: Vec<_> = key.collect();
                
                if path.len() < 2 {
                    continue;
                }
                
                let kind = match path.pop().unwrap() {
                    "u" => IndexKind::Unique,
                    "d" => IndexKind::Duplicate,
                    _ => continue,
                };
                
                let index_db = Database::open(
                    env.clone(), Some(db_name), &match kind {
                        IndexKind::Unique => DatabaseOptions::create_map::<str>(),
                        IndexKind::Duplicate => DatabaseOptions::create_multimap::<str, [u8;8]>(),
                    })
                    .wrap_err()?;
                
                let path = path.iter().flat_map(|s| once(".").chain(once(*s))).skip(1).collect();
                
                indexes.push(Arc::new(Index { path, kind, db: Arc::new(index_db) }));
            }
            
            Ok(indexes)
        }
    }
    
    pub fn add(&self, txn: &mut WriteTransaction, data: &Vec<(Binary, Vec<Binary>)>) -> Result<(), String> {
        if data.len() == 0 {
            return Ok(());
        }
        
        let txn = txn.child_tx().wrap_err()?;

        {
            let mut access = txn.access();
            let f = PutFlags::empty();
            
            for (key, vals) in data {
                for val in vals {
                    access.put(&self.db, &val.into_key(), &key.into_key(), f)
                      .wrap_err()?;
                }
            }
        }

        txn.commit().wrap_err()?;
        
        Ok(())
    }

    //pub fn del(&self, 

    pub fn extract(&self, doc: &Document) -> Vec<Binary> {
        let mut keys = Vec::new();
        let path = self.path.split('.');
        extract_field_values(doc, &path, &mut keys);
        keys
    }
}

fn extract_field_values<'a, 'i: 'a, I: Iterator<Item = &'i str> + Clone>(doc: &'a Document, path: &'a I, keys: &mut Vec<Binary>) {
    let mut sub_path = path.clone();
    if let Some(ref name) = sub_path.next() {
        use serde_cbor::Value::*;
        match doc {
            Array(val) => val.iter().for_each(|doc| extract_field_values(doc, path, keys)),
            Object(val) => if let Some(doc) = val.get(&document_field(*name)) {
                extract_field_values(doc, &sub_path, keys);
            },
            _ => (),
        }
    } else {
        extract_field_primitives(doc, keys);
    }
}

fn extract_field_primitives(doc: &Document, keys: &mut Vec<Binary>) {
    use serde_cbor::Value::*;
    match doc {
        U64(val) => keys.push(val.into_key()),
        I64(val) => keys.push(val.into_key()),
        Bytes(val) => keys.push(val.into_key()),
        String(val) => keys.push(val.into_key()),
        Bool(val) => keys.push(val.into_key()),
        Array(val) => val.iter().for_each(|doc| extract_field_primitives(doc, keys)),
        _ => (),
    }
}
