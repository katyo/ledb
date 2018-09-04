use std::iter::once;
use std::sync::{Arc, RwLock};
use std::collections::HashSet;
use serde::{Serialize, de::DeserializeOwned};
use ron::ser::to_string as to_db_name;
use serde_cbor::{self, Value, ObjectKey};
use lmdb::{Environment, put::Flags as PutFlags, Database, DatabaseOptions, ReadTransaction, WriteTransaction, Cursor, CursorIter, MaybeOwned, Unaligned};

use types::{Id, Document, ResultWrap, NOT_FOUND};
use index::{IndexDef, Index, IndexKind, IndexType};
use storage::{DatabaseDef};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CollectionDef (
    /// Collection name
    pub String,
);

impl CollectionDef {
    pub fn new<S: AsRef<str>>(name: S) -> Self {
        CollectionDef(name.as_ref().into())
    }
}

pub struct Collection {
    pub(crate) name: String,
    pub(crate) indexes: RwLock<Vec<Arc<Index>>>,
    pub(crate) env: Arc<Environment>,
    pub(crate) db: Arc<Database<'static>>,
}

impl Collection {
    pub fn new(env: Arc<Environment>, def: CollectionDef, index_defs: Vec<IndexDef>) -> Result<Self, String> {        
        let db_name = to_db_name(&DatabaseDef::Collection(def.clone())).wrap_err()?;
        
        let CollectionDef(name) = def;

        let db_opts = DatabaseOptions::create_map::<Unaligned<Id>>();
        
        let db = Arc::new(Database::open(
            env.clone(), Some(&db_name), &db_opts)
                          .wrap_err()?);
        
        let indexes = RwLock::new(
            index_defs
                .into_iter()
                .map(|def| Index::new(env.clone(), def).map(Arc::new))
                .collect::<Result<Vec<_>, _>>()
                .wrap_err()?
        );
        
        Ok(Self { name, indexes, env, db })
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
            
            access.put(&self.db, &Unaligned::new(id), &val, PutFlags::empty())
                  .wrap_err()?;
        }

        let indexes = self.indexes.read().wrap_err()?;

        for index in indexes.iter() {
            let vals = index.extract(&doc);
            index.add(&mut txn, &vec![(id, vals)])?;
        }

        txn.commit().wrap_err()?;

        Ok(id)
    }

    pub fn get<T: DeserializeOwned>(&self, id: Id) -> Result<Option<T>, String> {
        let txn = self.read_txn()?;
        let access = txn.access();

        match access.get::<Unaligned<Id>, [u8]>(&self.db, &Unaligned::new(id)) {
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
        
        Ok(match cursor.last::<Unaligned<Id>, [u8]>(&access) {
            Ok((key, _val)) => key.get(),
            Err(NOT_FOUND) => 0,
            Err(e) => return Err(e).wrap_err(),
        })
    }
    
    pub fn get_indexes(&self) -> Result<Vec<(String, IndexKind)>, String> {
        let indexes = self.indexes.read().wrap_err()?;
        Ok(indexes.iter().map(|index| (index.path.clone(), index.kind)).collect())
    }
    
    pub fn create_index<P: AsRef<str>>(&self, path: P, kind: IndexKind, key: IndexType) -> Result<bool, String> {
        let path = path.as_ref();
        
        {
            let indexes = self.indexes.read().wrap_err()?;
            if let Some(_) = indexes.iter().find(|index| index.path == path) {
                return Ok(false);
            }
        }
        
        let index = Index::new(self.env.clone(), IndexDef(self.name.clone(), path.into(), kind, key))
            .map(Arc::new)?;

        {
            let mut indexes = self.indexes.write().wrap_err()?;
            indexes.push(index.clone());
        }

        {
            let txn = self.read_txn()?;
            let cursor = txn.cursor(self.db.clone()).wrap_err()?;
            let access = txn.access();
            
            let data = CursorIter::new(MaybeOwned::Owned(cursor), &access,
                                       |c, a| c.first(a), Cursor::next::<Unaligned<Id>, [u8]>)
                .wrap_err()?
                .map(|res| res.wrap_err().and_then(|(key, val)| {
                    let doc = serde_cbor::from_slice(val).wrap_err()?;
                    Ok((key.get(), index.extract(&doc)))
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
