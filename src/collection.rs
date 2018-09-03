use std::iter::once;
use std::sync::{Arc, RwLock};
use std::collections::HashSet;
use serde::{Serialize, de::DeserializeOwned};
use serde_cbor::{self, Value, ObjectKey};
use lmdb::{Environment, put::Flags as PutFlags, Database, DatabaseOptions, ReadTransaction, WriteTransaction, Cursor, CursorIter, MaybeOwned};

pub use types::{Id, Document, Binary, ResultWrap, NOT_FOUND};
pub use key::{IntoKey, FromKey};
pub use index::{Index, IndexKind};

pub struct Collection {
    pub(crate) name: String,
    pub(crate) indexes: RwLock<Vec<Arc<Index>>>,
    pub(crate) env: Arc<Environment>,
    pub(crate) db: Arc<Database<'static>>,
}

impl Collection {
    pub fn bootstrap(env: Arc<Environment>, db: Arc<Database<'static>>) -> Result<Vec<Arc<Collection>>, String> {
        {
            let txn = ReadTransaction::new(env.clone()).wrap_err()?;
            let lst = {
                let cursor = txn.cursor(db.clone()).wrap_err()?;
                let access = txn.access();
                
                CursorIter::new(MaybeOwned::Owned(cursor), &access,
                                |c, a| c.first(a), Cursor::next::<str,[u8]>)
                    .wrap_err()?
                    .map(|res| res.map(|(key, _val)| key.split('.').next().unwrap()))
                    .collect::<Result<HashSet<_>, _>>()
                    .wrap_err()?
                    .iter().map(|s| (*s).into())
                           .collect::<Vec<String>>()
            };
            lst
        }.iter().cloned().map(move |name| {
            let indexes = RwLock::new(Index::bootstrap(env.clone(), db.clone(), &name)?);
            let collection_db = Database::open(
                env.clone(), Some(&name), &DatabaseOptions::create_map::<[u8;8]>())
                .wrap_err()?;
            Ok(Arc::new(Collection { name, indexes, env: env.clone(), db: Arc::new(collection_db) }))
        }).collect::<Result<Vec<_>, _>>()
    }
    
    pub fn insert<D: Serialize>(&self, doc: &D) -> Result<Id, String> {
        let doc = match serde_cbor::to_value(doc).wrap_err()? {
            Value::Object(mut val) => {
                val.remove(&ObjectKey::String("_id".into()));
                Value::Object(val)
            },
            _ => return Err("Document must be an object".into()),
        };
        
        let id = self.last_id()? + 1;

        let mut txn = self.write_txn()?;

        {
            let mut access = txn.access();
            let val = serde_cbor::to_vec(&doc).wrap_err()?;
            
            access.put(&self.db, &id.into_key(), &val, PutFlags::empty())
                  .wrap_err()?;
        }

        let indexes = self.indexes.read().wrap_err()?;

        for index in indexes.iter() {
            let vals = index.extract(&doc);
            index.add(&mut txn, &vec![(id.into_key(), vals)])?;
        }

        txn.commit().wrap_err()?;

        Ok(id)
    }

    pub fn get<T: DeserializeOwned>(&self, id: Id) -> Result<Option<T>, String> {
        let txn = self.read_txn()?;
        let access = txn.access();

        match access.get::<[u8], [u8]>(&self.db, &id.into_key()) {
            Ok(val) => match serde_cbor::from_slice(val).wrap_err()? {
                Value::Object(mut val) => {
                    val.insert(ObjectKey::String("_id".into()), Value::I64(id));
                    Ok(Some(serde_cbor::from_value(Value::Object(val))
                    .wrap_err()?))
                },
                _ => Err("Corrupted data".into())
            },
            Err(NOT_FOUND) => Ok(None),
            Err(e) => Err(e).wrap_err(),
        }
    }

    pub fn last_id(&self) -> Result<Id, String> {
        let txn = ReadTransaction::new(self.env.clone()).wrap_err()?;
        let mut cursor = txn.cursor(self.db.clone()).wrap_err()?;
        let access = txn.access();
        
        Ok(match cursor.last::<[u8], [u8]>(&access) {
            Ok((key, _val)) => Id::from_key(key),
            Err(NOT_FOUND) => 0,
            Err(e) => return Err(e).wrap_err(),
        })
    }
    
    pub fn get_indexes(&self) -> Result<Vec<(String, IndexKind)>, String> {
        let indexes = self.indexes.read().wrap_err()?;
        Ok(indexes.iter().map(|index| (index.path.clone(), index.kind)).collect())
    }
    
    pub fn create_index<P: AsRef<str>>(&self, path: P, kind: IndexKind) -> Result<bool, String> {
        let path = path.as_ref();
        
        {
            let indexes = self.indexes.read().wrap_err()?;
            if let Some(_) = indexes.iter().find(|index| index.path == path) {
                return Ok(false);
            }
        }
        
        let db_name: String = once(self.name.as_str())
            .chain(once("."))
            .chain(once(path))
            .chain(once(match kind {
                IndexKind::Unique => ".u",
                IndexKind::Duplicate => ".d",
            })).collect();

        let index_db = Database::open(
            self.env.clone(), Some(&db_name), &match kind {
                IndexKind::Unique => DatabaseOptions::create_map::<str>(),
                IndexKind::Duplicate => DatabaseOptions::create_multimap::<str, [u8;8]>(),
            })
            .wrap_err()?;

        let path = path.into();
        let index = Arc::new(Index { path, kind, db: Arc::new(index_db) });

        {
            let mut indexes = self.indexes.write().wrap_err()?;
            indexes.push(index.clone());
        }

        {
            let txn = self.read_txn()?;
            let cursor = txn.cursor(self.db.clone()).wrap_err()?;
            let access = txn.access();
            
            let data = CursorIter::new(MaybeOwned::Owned(cursor), &access,
                                       |c, a| c.first(a), Cursor::next::<[u8], [u8]>)
                .wrap_err()?
                .map(|res| res.wrap_err().and_then(|(key, val)| {
                    let doc = serde_cbor::from_slice(val).wrap_err()?;
                    Ok((Binary::from_key(key), index.extract(&doc)))
                }))
                .collect::<Result<Vec<_>, _>>()
                .wrap_err()?;

            let mut txn = self.write_txn()?;
            
            index.add(&mut txn, &data).wrap_err()?;

            txn.commit().wrap_err()?;
        }
        
        Ok(true)
    }

    pub fn remove_index<P: AsRef<str>>(&self, path: P) -> Result<bool, String> {
        let path = path.as_ref();
        
        let mut indexes = self.indexes.write().wrap_err()?;
        
        let index_pos = if let Some(index) = indexes.iter().position(|index| index.path == path) {
            index
        } else {
            return Ok(false);
        };

        {
            //let index = &indexes[index_pos];
            //let index_db = &index.db;
            //index_db.delete().wrap_err()?;
        }

        indexes.remove(index_pos);
        
        //index.db.delete().wrap_err()?;

        Ok(true)
    }

    fn read_txn(&self) -> Result<ReadTransaction, String> {
        ReadTransaction::new(self.env.clone()).wrap_err()
    }

    fn write_txn(&self) -> Result<WriteTransaction, String> {
        WriteTransaction::new(self.env.clone()).wrap_err()
    }
}
