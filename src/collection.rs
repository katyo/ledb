use std::iter::once;
use std::sync::{Arc, RwLock};
use std::collections::HashSet;
use serde::{Serialize, de::DeserializeOwned};
use ron::ser::to_string as to_db_name;
use serde_cbor::{self, Value, ObjectKey};
use lmdb::{Environment, put::Flags as PutFlags, Database, DatabaseOptions, ReadTransaction, WriteTransaction, Cursor, CursorIter, MaybeOwned, Unaligned, LmdbResultExt};

use error::{Error, Result, ResultWrap};
use document::{Primary, Document};
use index::{IndexDef, Index, IndexKind};
use filter::{Filter, Cond, Comp, KeyType};
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
    pub fn new(env: Arc<Environment>, def: CollectionDef, index_defs: Vec<IndexDef>) -> Result<Self> {        
        let db_name = to_db_name(&DatabaseDef::Collection(def.clone())).wrap_err()?;
        
        let CollectionDef(name) = def;

        let db_opts = DatabaseOptions::create_map::<Unaligned<Primary>>();
        
        let db = Arc::new(Database::open(
            env.clone(), Some(&db_name), &db_opts)
                          .wrap_err()?);
        
        let indexes = RwLock::new(
            index_defs
                .into_iter()
                .map(|def| Index::new(env.clone(), def).map(Arc::new))
                .collect::<Result<Vec<_>>>()?
        );
        
        Ok(Self { name, indexes, env, db })
    }
    
    pub fn insert<T: Serialize>(&self, doc: &T) -> Result<Primary> {
        let id = self.last_id()? + 1;

        self.put(&Document::new(doc).with_id(id))?;

        Ok(id)
    }

    pub fn find(&self, filter: Filter) -> /*ListIterator*/ Result<HashSet<Primary>> {
        let txn = self.read_txn()?;
        let access = txn.access();
        
        match filter {
            Filter::Comp(path, Comp::Eq(val)) => {
                if let Some(index) = self.get_index(path)? {
                    return Ok(index.query_set(&txn, &access, once(&val))?)
                }
            },
            _ => (),
        }

        Ok(HashSet::new())
    }

    pub fn has(&self, id: Primary) -> Result<bool> {
        let txn = self.read_txn()?;
        let access = txn.access();

        access.get::<Unaligned<Primary>, [u8]>(&self.db, &Unaligned::new(id))
            .to_opt().map(|res| res != None).wrap_err()
    }

    pub fn get<T: DeserializeOwned>(&self, id: Primary) -> Result<Option<Document<T>>> {
        let txn = self.read_txn()?;
        let access = txn.access();

        Ok(match access.get::<Unaligned<Primary>, [u8]>(&self.db, &Unaligned::new(id))
            .to_opt().wrap_err()? {
                Some(val) => Some(Document::from_raw(val)?.with_id(id)),
                None => None,
            })
    }

    pub fn put<T: Serialize>(&self, doc: &Document<T>) -> Result<()> {
        if !doc.has_id() {
            return Err("Document id is missing".into());
        }

        let id = doc.get_id().unwrap();
        let doc = doc.into_gen()?;
        
        let txn = self.write_txn()?;

        {
            let mut access = txn.access();
            let val = doc.into_raw()?;
            
            access.put(&self.db, &Unaligned::new(id), &val, PutFlags::empty())
                  .wrap_err()?;
        }

        self.remove_from_indexes(&txn, id)?;
        self.add_to_indexes(&txn, &doc)?;

        txn.commit().wrap_err()?;

        Ok(())
    }

    fn remove_from_indexes(&self, txn: &WriteTransaction, old_id: Primary) -> Result<()> {
        if let Some(old_doc) = self.get(old_id)? {
            let indexes = self.indexes.read().wrap_err()?;

            let mut access = txn.access();
            
            for index in indexes.iter() {
                index.remove_from_index(&mut access, &old_doc)?;
            }
        }

        Ok(())
    }

    fn add_to_indexes(&self, txn: &WriteTransaction, new_doc: &Document) -> Result<()> {
        let indexes = self.indexes.read().wrap_err()?;

        let mut access = txn.access();
        
        for index in indexes.iter() {
            index.add_to_index(&mut access, &new_doc)?;
        }

        Ok(())
    }

    pub fn last_id(&self) -> Result<Primary> {
        let txn = ReadTransaction::new(self.env.clone()).wrap_err()?;
        let mut cursor = txn.cursor(self.db.clone()).wrap_err()?;
        let access = txn.access();
        
        cursor.last::<Unaligned<Primary>, [u8]>(&access)
            .to_opt().map(|res| res.map(|(key, _val)| key.get()).unwrap_or(0)).wrap_err()
    }
    
    pub fn get_indexes(&self) -> Result<Vec<(String, IndexKind)>> {
        let indexes = self.indexes.read().wrap_err()?;
        Ok(indexes.iter().map(|index| (index.path.clone(), index.kind)).collect())
    }
    
    pub fn create_index<P: AsRef<str>>(&self, path: P, kind: IndexKind, key: KeyType) -> Result<bool> {
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
            let txn = self.write_txn()?;
            {
                let mut access = txn.access();

                let txn2 = self.read_txn()?;
                let cursor2 = txn2.cursor(self.db.clone()).wrap_err()?;
                let access2 = txn2.access();
                
                for res in CursorIter::new(MaybeOwned::Owned(cursor2), &access2,
                                           |c, a| c.first(a), Cursor::next::<Unaligned<Primary>, [u8]>)
                    .wrap_err()?
                {
                    let (key, val) = res.wrap_err()?;
                    let doc = Document::from_raw(val)?.with_id(key.get());
                    index.add_to_index(&mut access, &doc)?;
                }
            }

            txn.commit().wrap_err()?;
        }
        
        Ok(true)
    }

    pub fn remove_index<P: AsRef<str>>(&self, path: P) -> Result<bool> {
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

    fn get_index<P: AsRef<str>>(&self, path: P) -> Result<Option<Arc<Index>>> {
        let path = path.as_ref();
        let indexes = self.indexes.read().wrap_err()?;
        
        Ok(indexes.iter().find(|index| index.path == path).map(Clone::clone))
    }

    fn read_txn(&self) -> Result<ReadTransaction> {
        ReadTransaction::new(self.env.clone()).wrap_err()
    }

    fn write_txn(&self) -> Result<WriteTransaction> {
        WriteTransaction::new(self.env.clone()).wrap_err()
    }
}
