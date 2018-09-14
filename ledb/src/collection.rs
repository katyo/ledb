use std::sync::{Arc, RwLock};
use std::marker::PhantomData;
use std::collections::HashSet;
use serde::{Serialize, de::DeserializeOwned};
use ron::ser::to_string as to_db_name;
use lmdb::{Environment, put::Flags as PutFlags, Database, DatabaseOptions, ReadTransaction, WriteTransaction, Cursor, CursorIter, MaybeOwned, Unaligned, LmdbResultExt, traits::CreateCursor};

use super::{Result, ResultWrap, KeyType, Primary, Document, Value, IndexDef, Index, IndexKind, Filter, Order, OrderKind, DatabaseDef, Modify};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct CollectionDef (
    /// Collection name
    pub String,
);

impl CollectionDef {
    pub fn new<S: AsRef<str>>(name: S) -> Self {
        CollectionDef(name.as_ref().into())
    }
}

/// Collection of documents
pub struct Collection {
    pub(crate) name: String,
    pub(crate) indexes: RwLock<Vec<Arc<Index>>>,
    pub(crate) env: Arc<Environment>,
    pub(crate) db: Arc<Database<'static>>,
}

impl Collection {
    pub(crate) fn new(env: Arc<Environment>, def: CollectionDef, index_defs: Vec<IndexDef>) -> Result<Self> {        
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

    /// Insert document into collection
    ///
    /// The primary key/identifier of document will be selected by auto incrementing the id of last inserted document.
    ///
    /// Primary key/identifier of new inserted document will be returned.
    ///
    pub fn insert<T: Serialize>(&self, doc: &T) -> Result<Primary> {
        let id = self.new_id()?;

        self.put(&Document::new(doc).with_id(id))?;

        Ok(id)
    }

    /// Find documents using optional filter and ordering
    ///
    /// When none filter specified then all documents will be found.
    ///
    /// Iterator across found documents will be returned.
    ///
    /// You can use `DocumentsIterator::size_hint()` for getting the total number of found documents.
    ///
    pub fn find<T: DeserializeOwned>(&self, filter: Option<Filter>, order: Order) -> Result<DocumentsIterator<T>> {
        let txn = Arc::new(ReadTransaction::new(self.env.clone())?);

        let all_ids: Box<Iterator<Item = Result<Primary>>> = match order {
            Order::Primary(order) => Box::new(PrimaryIterator::new(txn.clone(), self.db.clone(), order)?),
            Order::Field(path, order) => Box::new(self.req_index(path)?.query_iter(txn.clone(), order)?),
        };
        
        let filtered_ids =
            if let Some(filter) = filter {
                let sel = filter.apply(&txn, &self)?;
                all_ids.filter(move |res| if let Ok(id) = res {
                    sel.has(&id)
                } else {
                    true
                }).collect::<Result<Vec<_>>>()?
            } else {
                all_ids.collect::<Result<Vec<_>>>()?
            };
        
        Ok(DocumentsIterator::new(self.env.clone(), self.db.clone(), filtered_ids)?)
    }

    /// Find documents using optional filter and ordering
    ///
    /// When none filter specified then all documents will be found.
    ///
    /// The vector with found documents will be returned.
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

    /// Update documents using optional filter and modifier
    ///
    /// *Note*: When none filter specified then all documents will be modified.
    ///
    /// Returns the number of affected documents.
    ///
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

    /// Remove documents using optional filter
    ///
    /// *Note*: When none filter specified then all documents will be removed.
    ///
    /// Returns the number of affected documents.
    ///
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

    /// Dump all documents which stored into the collection
    #[inline]
    pub fn dump<T: DeserializeOwned>(&self) -> Result<DocumentsIterator<T>> {
        self.find(None, Order::default())
    }

    /// Load new documents into the collection
    ///
    /// *Note*: The old documents will be removed.
    ///
    pub fn load<T: Serialize, I>(&self, docs: I) -> Result<usize>
        where I: IntoIterator<Item = Document<T>>
    {
        self.purge()?;

        let txn = WriteTransaction::new(self.env.clone())?;
        let f = PutFlags::empty();
        let mut count = 0;

        {
            for doc in docs.into_iter() {
                let id = doc.req_id()?;
                let doc = doc.into_gen()?;
                
                {
                    let mut access = txn.access();
                    
                    access.put(&self.db, &Unaligned::new(id), &doc.into_raw()?, f)
                          .wrap_err()?;
                }

                self.update_indexes(&txn, None, Some(&doc))?;

                count += 1;
            }
        }

        Ok(count)
    }

    /// Remove all documents from the collection
    ///
    /// Shortcut for `Collection::remove(None)`.
    ///
    #[inline]
    pub fn purge(&self) -> Result<usize> {
        self.remove(None)
    }

    /// Checks the collection contains document with specified primary key
    pub fn has(&self, id: Primary) -> Result<bool> {
        let txn = ReadTransaction::new(self.env.clone()).wrap_err()?;
        let access = txn.access();

        access.get::<Unaligned<Primary>, [u8]>(&self.db, &Unaligned::new(id))
            .to_opt().map(|res| res != None).wrap_err()
    }

    /// Get document from collection using primary key/identifier
    pub fn get<T: DeserializeOwned>(&self, id: Primary) -> Result<Option<Document<T>>> {
        let txn = ReadTransaction::new(self.env.clone()).wrap_err()?;
        let access = txn.access();

        Ok(match access.get::<Unaligned<Primary>, [u8]>(&self.db, &Unaligned::new(id))
            .to_opt().wrap_err()? {
                Some(val) => Some(Document::<T>::from_raw(val)?.with_id(id)),
                None => None,
            })
    }

    /// Replace document in the collection
    ///
    /// *Note*: The document must have primary key/identifier.
    ///
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

    /// Delete document with specified primary key/identifier from the collection
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

    /// Get the last primary key/identifier of inserted document
    pub fn last_id(&self) -> Result<Primary> {
        let txn = ReadTransaction::new(self.env.clone()).wrap_err()?;
        let mut cursor = txn.cursor(self.db.clone()).wrap_err()?;
        let access = txn.access();
        
        cursor.last::<Unaligned<Primary>, [u8]>(&access)
            .to_opt().map(|res| res.map(|(key, _val)| key.get()).unwrap_or(0)).wrap_err()
    }

    /// Get the new primary key/identifier
    pub fn new_id(&self) -> Result<Primary> {
        self.last_id().map(|id| id + 1)
    }

    /// Get indexes info from the collection
    pub fn get_indexes(&self) -> Result<Vec<(String, IndexKind, KeyType)>> {
        let indexes = self.indexes.read().wrap_err()?;
        Ok(indexes.iter().map(|index| (index.path.clone(), index.kind, index.key)).collect())
    }

    /// Ensure index for the collection
    pub fn ensure_index<P: AsRef<str>>(&self, path: P, kind: IndexKind, key: KeyType) -> Result<bool> {
        if let Some(index) = self.get_index(&path)? {
            if index.kind == kind && index.key == key {
                return Ok(false);
            } else {
                self.remove_index(&path)?;
            }
        }

        self.create_index(&path, kind, key)
    }

    /// Create index for the collection
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

    /// Remove index from the collection
    pub fn remove_index<P: AsRef<str>>(&self, path: P) -> Result<bool> {
        let path = path.as_ref();
        
        let mut indexes = self.indexes.write().wrap_err()?;
        
        let index_pos = if let Some(index) = indexes.iter().position(|index| index.path == path) {
            index
        } else {
            return Ok(false);
        };

        let index = indexes.remove(index_pos);

        if let Ok(Index { db, .. }) = Arc::try_unwrap(index) {
            if let Ok(db) = Arc::try_unwrap(db) {
                db.delete().wrap_err()?;
            }
        }
        
        Ok(true)
    }

    /// Checks the index for specified field exists for the collection
    pub fn has_index<P: AsRef<str>>(&self, path: P) -> Result<bool> {
        let path = path.as_ref();
        let indexes = self.indexes.read().wrap_err()?;

        Ok(indexes.iter().any(|index| index.path == path))
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

pub(crate) struct PrimaryIterator {
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

/// Iterator across found documents
///
/// You can use that to extract documents contents
///
/// The `DocumentsIterator::size_hint()` method gets actual number of found documents.
///
pub struct DocumentsIterator<T> {
    env: Arc<Environment>,
    db: Arc<Database<'static>>,
    ids_iter: Box<Iterator<Item = Primary> + Send>,
    phantom_doc: PhantomData<T>,
}

impl<T> DocumentsIterator<T> {
    pub fn new<I>(env: Arc<Environment>, db: Arc<Database<'static>>, ids_iter: I) -> Result<Self>
    where I: IntoIterator<Item = Primary> + 'static,
          I::IntoIter: Send,
    {
        Ok(Self { env, db, ids_iter: Box::new(ids_iter.into_iter()), phantom_doc: PhantomData })
    }
}

impl<T> Iterator for DocumentsIterator<T>
    where T: DeserializeOwned
{
    type Item = Result<Document<T>>;

    fn next(&mut self) -> Option<Self::Item> {
        self.ids_iter
            .next().map(|id| {
                let txn = ReadTransaction::new(self.env.clone())?;
                {
                    let access = txn.access();
                    access.get(&self.db, &Unaligned::new(id))
                          .wrap_err()
                          .and_then(Document::<T>::from_raw)
                          .map(|doc| doc.with_id(id))
                          .wrap_err()
                }
            })
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.ids_iter.size_hint()
    }
}
