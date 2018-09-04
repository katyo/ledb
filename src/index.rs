use std::sync::{Arc};
//use std::iter::once;
use std::collections::HashSet;
use ron::ser::to_string as to_db_name;
use lmdb::{Environment, put::Flags as PutFlags, Database, DatabaseOptions, ReadTransaction, ConstAccessor, WriteAccessor, Unaligned, MaybeOwned, Cursor, CursorIter, LmdbResultExt};

use types::{document_field, ResultWrap};
use document::{Primary, Document, Value};
use storage::{DatabaseDef};
use filter::{KeyType, KeyData};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum IndexKind {
    #[serde(rename="uni")]
    Unique,
    #[serde(rename="dup")]
    Duplicate,
}

impl Default for IndexKind {
    fn default() -> Self { IndexKind::Duplicate }
}


/*
#[derive(Debug, Clone)]
pub enum IndexQuery {
    Set(Vec<IndexData>),
    Range(Option<IndexData>, Option<IndexData>),
}
*/

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IndexDef (
    /// Collection name
    pub String,
    /// Field path
    pub String,
    pub IndexKind,
    pub KeyType,
);

pub struct Index {
    pub(crate) path: String,
    pub(crate) kind: IndexKind,
    pub(crate) key: KeyType,
    pub(crate) db: Arc<Database<'static>>,
}

impl Index {
    pub fn new(env: Arc<Environment>, def: IndexDef) -> Result<Self, String> {
        let db_name = to_db_name(&DatabaseDef::Index(def.clone())).wrap_err()?;
        
        let IndexDef(_coll, path, kind, key) = def;

        let db_opts = match (kind, key) {
            (IndexKind::Unique, KeyType::Int) => DatabaseOptions::create_map::<Unaligned<i64>>(),
            //(IndexKind::Unique, KeyType::Float) => DatabaseOptions::create_map::<Unaligned<f64>>(),
            (IndexKind::Unique, KeyType::String) => DatabaseOptions::create_map::<str>(),
            (IndexKind::Unique, KeyType::Binary) => DatabaseOptions::create_map::<[u8]>(),
            (IndexKind::Unique, KeyType::Bool) => DatabaseOptions::create_map::<u8>(),
            (IndexKind::Duplicate, KeyType::Int) => DatabaseOptions::create_multimap::<Unaligned<i64>, Unaligned<Primary>>(),
            //(IndexKind::Duplicate, KeyType::Float) => DatabaseOptions::create_multimap::<Unaligned<f64>, Unaligned<Primary>>(),
            (IndexKind::Duplicate, KeyType::String) => DatabaseOptions::create_multimap::<str, Unaligned<Primary>>(),
            (IndexKind::Duplicate, KeyType::Binary) => DatabaseOptions::create_multimap::<[u8], Unaligned<Primary>>(),
            (IndexKind::Duplicate, KeyType::Bool) => DatabaseOptions::create_multimap::<u8, Unaligned<Primary>>(),
            _ => unimplemented!(),
        };
        
        let db = Arc::new(Database::open(
            env.clone(), Some(&db_name), &db_opts)
                          .wrap_err()?);
        
        Ok(Self { path, kind, key, db })
    }

    pub fn add_to_index(&self, access: &mut WriteAccessor, doc: &Document) -> Result<(), String> {
        let id = doc.get_id().ok_or_else(|| "Missing document id".to_string())?;
        let f = PutFlags::empty();
        
        for key in self.extract(doc) {
            access.put(&self.db, key.into_raw(), &Unaligned::new(id), f)
                  .wrap_err()?;
        }

        Ok(())
    }

    pub fn remove_from_index(&self, access: &mut WriteAccessor, doc: &Document) -> Result<(), String> {
        let id = doc.get_id().ok_or_else(|| "Missing document id".to_string())?;
        
        for key in self.extract(doc) {
            access.del_item(&self.db, key.into_raw(), &Unaligned::new(id))
                  .wrap_err()?;
        }

        Ok(())
    }

    fn extract(&self, doc: &Document) -> Vec<KeyData> {
        let mut keys = Vec::new();
        let path = self.path.split('.');
        extract_field_values(doc.get_data(), &self.key, &path, &mut keys);
        keys
    }

    pub fn query_set<'a, I: Iterator<Item = &'a KeyData>>(&self, txn: &ReadTransaction, access: &ConstAccessor, keys: I) -> Result<HashSet<Primary>, String> {
        let mut out = HashSet::new();
        
        for key in keys {
            if let Some(key) = key.cast_type(&self.key) {
                let mut cursor = txn.cursor(self.db.clone()).wrap_err()?;

                match self.kind {
                    IndexKind::Unique => {
                        match cursor.seek_k_both::<[u8], Unaligned<Primary>>(&access, key.into_raw()).to_opt() {
                            Ok(Some((_key, id))) => { out.insert(id.get()); },
                            Err(e) => return Err(e).wrap_err(),
                            _ => (),
                        }
                    },
                    IndexKind::Duplicate => {
                        match cursor.seek_k::<[u8], Unaligned<Primary>>(&access, key.into_raw()).to_opt() {
                            Ok(Some(..)) => (),
                            Ok(None) => continue,
                            Err(e) => return Err(e).wrap_err(),
                        }
                        
                        for res in CursorIter::new(MaybeOwned::Owned(cursor), &access,
                                                   |c, a| c.get_multiple::<[Unaligned<Primary>]>(&a),
                                                   Cursor::next_multiple::<[Unaligned<Primary>]>)
                            .wrap_err()?
                        {
                            if let Some(ids) = res.to_opt().wrap_err()? {
                                for id in ids {
                                    out.insert(id.get());
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok(out)
    }
}

fn extract_field_values<'a, 'i: 'a, I: Iterator<Item = &'i str> + Clone>(doc: &'a Value, typ: &KeyType, path: &'a I, keys: &mut Vec<KeyData>) {
    let mut sub_path = path.clone();
    if let Some(ref name) = sub_path.next() {
        use serde_cbor::Value::*;
        match doc {
            Array(val) => val.iter().for_each(|doc| extract_field_values(doc, typ, path, keys)),
            Object(val) => if let Some(doc) = val.get(&document_field(*name)) {
                extract_field_values(doc, typ, &sub_path, keys);
            },
            _ => (),
        }
    } else {
        extract_field_primitives(doc, typ, keys);
    }
}

fn extract_field_primitives(doc: &Value, typ: &KeyType, keys: &mut Vec<KeyData>) {
    use serde_cbor::Value::*;
    match (typ, doc) {
        (_, Array(val)) => val.iter().for_each(|doc| extract_field_primitives(doc, typ, keys)),
        (typ, val) => {
            if let Some(val) = KeyData::from_val(&val) {
                if let Some(val) = val.cast_type(typ) {
                    keys.push(val.clone());
                }
            }
        },
    }
}
