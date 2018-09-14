use std::sync::{Arc, RwLock};
use std::marker::PhantomData;
use std::collections::HashSet;
use serde::{Serialize, de::DeserializeOwned};
use ron::ser::to_string as to_db_name;
use lmdb::{Environment, put::Flags as PutFlags, Database, DatabaseOptions, ReadTransaction, WriteTransaction, Cursor, CursorIter, MaybeOwned, Unaligned, LmdbResultExt, traits::CreateCursor};

use super::{Result, ResultWrap, KeyType, Primary, Document, Value, IndexDef, Index, IndexKind, Filter, Order, OrderKind, DatabaseDef, Modify};

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
        let id = self.new_id()?;

        self.put(&Document::new(doc).with_id(id))?;

        Ok(id)
    }

    pub fn find<T: DeserializeOwned>(&self, filter: Option<Filter>, order: Order) -> Result<DocumentsIterator<T>> {
        let txn = Arc::new(ReadTransaction::new(self.env.clone())?);

        let all_ids: Box<Iterator<Item = Result<Primary>>> = match order {
            Order::Primary(order) => Box::new(PrimaryIterator::new(txn.clone(), self.db.clone(), order)?),
            Order::Field(path, order) => Box::new(self.req_index(path)?.query_iter(txn.clone(), order)?),
        };
        
        let filtered_ids: Box<Iterator<Item = Result<Primary>>> =
            if let Some(filter) = filter {
                let sel = filter.apply(&txn, &self)?;
                Box::new(all_ids.filter(move |res| if let Ok(id) = res {
                    sel.has(&id)
                } else {
                    true
                }))
            } else {
                Box::new(all_ids)
            };
        
        Ok(DocumentsIterator::new(txn.clone(), self.db.clone(), filtered_ids)?)
    }

    pub fn find_all<T: DeserializeOwned>(&self, filter: Option<Filter>, order: Order) -> Result<Vec<Document<T>>> {
        self.find(filter, order)?.collect::<Result<Vec<_>>>()
    }

    pub fn find_ids(&self, filter: Option<Filter>) -> Result<HashSet<Primary>> {
        let txn = Arc::new(ReadTransaction::new(self.env.clone())?);
            
        if let Some(filter) = filter {
            let sel = filter.apply(&txn, &self)?;
            if !sel.inv {
                Ok(sel.ids)
            } else {
                PrimaryIterator::new(txn, self.db.clone(), OrderKind::default())?
                    .filter(move |res| if let Ok(id) = res { sel.has(id) } else { true })
                    .collect::<Result<HashSet<_>>>()
            }
        } else {
            PrimaryIterator::new(txn, self.db.clone(), OrderKind::default())?
                .collect::<Result<HashSet<_>>>()
        }
    }

    pub fn update(&self, filter: Option<Filter>, modify: Modify) -> Result<usize> {
        let found_ids = self.find_ids(filter)?;
            
        let mut count = 0;
        {
            let txn = WriteTransaction::new(self.env.clone())?;
            let f = PutFlags::empty();
            {
                for id in found_ids {
                    let (old_doc, new_doc) = {
                        let mut access = txn.access();
                        let old_doc = Document::<Value>::from_raw(access.get(&self.db, &Unaligned::new(id))?)?.with_id(id);
                        let new_doc = Document::new(modify.apply(old_doc.get_data().clone()));
                        
                        access.put(&self.db, &Unaligned::new(id), &new_doc.into_raw()?, f)
                              .wrap_err()?;
                        
                        (old_doc, new_doc)
                    };
                    
                    self.update_indexes(&txn, Some(&old_doc), Some(&new_doc))?;
                    
                    count += 1;
                }
            }
            
            txn.commit().wrap_err()?;
        }

        Ok(count)
    }

    pub fn remove(&self, filter: Option<Filter>) -> Result<usize> {
        let found_ids = self.find_ids(filter)?;
            
        let mut count = 0;
        {
            let txn = WriteTransaction::new(self.env.clone())?;
            {
                for id in found_ids {
                    let old_doc = {
                        let mut access = txn.access();
                        let old_doc = Document::<Value>::from_raw(access.get(&self.db, &Unaligned::new(id))?)?.with_id(id);
                        
                        access.del_key(&self.db, &Unaligned::new(id))
                              .wrap_err()?;
                        
                        old_doc
                    };

                    self.update_indexes(&txn, Some(&old_doc), None)?;
                    
                    count += 1;
                }
            }
            
            txn.commit().wrap_err()?;
        }

        Ok(count)
    }

    pub fn has(&self, id: Primary) -> Result<bool> {
        let txn = ReadTransaction::new(self.env.clone()).wrap_err()?;
        let access = txn.access();

        access.get::<Unaligned<Primary>, [u8]>(&self.db, &Unaligned::new(id))
            .to_opt().map(|res| res != None).wrap_err()
    }

    pub fn get<T: DeserializeOwned>(&self, id: Primary) -> Result<Option<Document<T>>> {
        let txn = ReadTransaction::new(self.env.clone()).wrap_err()?;
        let access = txn.access();

        Ok(match access.get::<Unaligned<Primary>, [u8]>(&self.db, &Unaligned::new(id))
            .to_opt().wrap_err()? {
                Some(val) => Some(Document::<T>::from_raw(val)?.with_id(id)),
                None => None,
            })
    }

    pub fn put<T: Serialize>(&self, doc: &Document<T>) -> Result<()> {
        if !doc.has_id() {
            return Err("Document id is missing".into());
        }

        let id = doc.get_id().unwrap();
        let doc = doc.into_gen()?;
        
        let txn = WriteTransaction::new(self.env.clone()).wrap_err()?;

        let old_doc = {
            let mut access = txn.access();
            let old_doc = if let Some(old_doc) = access.get(&self.db, &Unaligned::new(id)).to_opt()? {
                Some(Document::<Value>::from_raw(old_doc)?.with_id(id))
            } else {
                None
            };
            
            access.put(&self.db, &Unaligned::new(id), &doc.into_raw()?, PutFlags::empty())
                  .wrap_err()?;

            old_doc
        };

        self.update_indexes(&txn, if let Some(ref doc) = old_doc { Some(&doc) } else { None }, Some(&doc))?;

        txn.commit().wrap_err()?;

        Ok(())
    }

    pub fn delete(&self, id: Primary) -> Result<bool> {
        let txn = WriteTransaction::new(self.env.clone()).wrap_err()?;

        let old_doc = {
            let mut access = txn.access();
            let old_doc = Document::<Value>::from_raw(access.get(&self.db, &Unaligned::new(id))?)?.with_id(id);
            
            access.del_key(&self.db, &Unaligned::new(id))
                  .wrap_err()?;

            old_doc
        };

        let status = self.update_indexes(&txn, Some(&old_doc), None)?;

        txn.commit().wrap_err()?;

        Ok(status)
    }

    fn update_indexes(&self, txn: &WriteTransaction, old_doc: Option<&Document>, new_doc: Option<&Document>) -> Result<bool> {
        {
            let indexes = self.indexes.read().wrap_err()?;
            let mut access = txn.access();
            
            for index in indexes.iter() {
                index.update_index(&mut access, old_doc, new_doc)?;
            }
        }
        
        Ok(old_doc.is_some())
    }

    pub fn last_id(&self) -> Result<Primary> {
        let txn = ReadTransaction::new(self.env.clone()).wrap_err()?;
        let mut cursor = txn.cursor(self.db.clone()).wrap_err()?;
        let access = txn.access();
        
        cursor.last::<Unaligned<Primary>, [u8]>(&access)
            .to_opt().map(|res| res.map(|(key, _val)| key.get()).unwrap_or(0)).wrap_err()
    }

    pub fn new_id(&self) -> Result<Primary> {
        self.last_id().map(|id| id + 1)
    }
    
    pub fn get_indexes(&self) -> Result<Vec<(String, IndexKind, KeyType)>> {
        let indexes = self.indexes.read().wrap_err()?;
        Ok(indexes.iter().map(|index| (index.path.clone(), index.kind, index.key)).collect())
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
            let txn = WriteTransaction::new(self.env.clone()).wrap_err()?;
            {
                let mut access = txn.access();

                let txn2 = ReadTransaction::new(self.env.clone()).wrap_err()?;
                let cursor2 = txn2.cursor(self.db.clone()).wrap_err()?;
                let access2 = txn2.access();
                
                for res in CursorIter::new(MaybeOwned::Owned(cursor2), &access2,
                                           |c, a| c.first(a), Cursor::next::<Unaligned<Primary>, [u8]>)
                    .wrap_err()?
                {
                    let (key, val) = res.wrap_err()?;
                    let doc = Document::<Value>::from_raw(val)?.with_id(key.get());
                    index.update_index(&mut access, None, Some(&doc))?;
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

    pub(crate) fn get_index<P: AsRef<str>>(&self, path: P) -> Result<Option<Arc<Index>>> {
        let path = path.as_ref();
        let indexes = self.indexes.read().wrap_err()?;
        
        Ok(indexes.iter().find(|index| index.path == path).map(Clone::clone))
    }

    pub(crate) fn req_index<P: AsRef<str>>(&self, path: P) -> Result<Arc<Index>> {
        if let Some(index) = self.get_index(&path)? {
            Ok(index)
        } else {
            Err(format!("Missing index for field '{}'", path.as_ref())).wrap_err()
        }
    }
}

pub struct PrimaryIterator {
    txn: Arc<ReadTransaction<'static>>,
    cur: Cursor<'static, 'static>,
    order: OrderKind,
    init: bool,
}

impl PrimaryIterator {
    pub fn new(txn: Arc<ReadTransaction<'static>>, db: Arc<Database<'static>>, order: OrderKind) -> Result<Self> {
        let cur = txn.cursor(db)?;

        Ok(Self { txn, cur, order, init: false })
    }
}

impl Iterator for PrimaryIterator {
    type Item = Result<Primary>;

    fn next(&mut self) -> Option<Self::Item> {
        let access = self.txn.access();
        match if self.init {
            match self.order {
                OrderKind::Asc => self.cur.next::<Unaligned<Primary>, [u8]>(&access),
                OrderKind::Desc => self.cur.prev::<Unaligned<Primary>, [u8]>(&access),
            }
        } else {
            self.init = true;
            match self.order {
                OrderKind::Asc => self.cur.first::<Unaligned<Primary>, [u8]>(&access),
                OrderKind::Desc => self.cur.last::<Unaligned<Primary>, [u8]>(&access),
            }
        }.to_opt() {
            Ok(Some((id, _val))) => Some(Ok(id.get())),
            Ok(None) => None,
            Err(e) => Some(Err(e).wrap_err()),
        }
    }
}

pub struct DocumentsIterator<T> {
    db: Arc<Database<'static>>,
    txn: Arc<ReadTransaction<'static>>,
    ids_iter: Box<Iterator<Item = Result<Primary>>>,
    phantom_doc: PhantomData<T>,
}

impl<T> DocumentsIterator<T> {
    pub fn new(txn: Arc<ReadTransaction<'static>>, db: Arc<Database<'static>>, ids_iter: Box<Iterator<Item = Result<Primary>>>) -> Result<Self> {
        Ok(Self { db, txn, ids_iter, phantom_doc: PhantomData })
    }
}

impl<T> Iterator for DocumentsIterator<T>
    where T: DeserializeOwned
{
    type Item = Result<Document<T>>;

    fn next(&mut self) -> Option<Self::Item> {
        match self.ids_iter.next() {
            Some(Ok(id)) => Some(self.txn.access().get(&self.db, &Unaligned::new(id)).wrap_err()
                                   .and_then(Document::<T>::from_raw).map(|doc| doc.with_id(id)).wrap_err()),
            Some(Err(e)) => Some(Err(e)),
            None => None,
        }
    }
}
