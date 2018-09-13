use std::sync::Arc;
use std::borrow::Cow;
use std::collections::HashSet;
use ron::ser::to_string as to_db_name;
use lmdb::{Environment, put::{NOOVERWRITE, NODUPDATA}, Database, DatabaseOptions, ReadTransaction, ConstAccessor, WriteAccessor, Unaligned, MaybeOwned, Cursor, CursorIter, LmdbResultExt, traits::CreateCursor};

use super::{Result, ResultWrap, Primary, Document, Value, DatabaseDef, KeyType, KeyData, OrderKind};
use extra::CursorExtra;
use float::F64;

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
    pub fn new(env: Arc<Environment>, def: IndexDef) -> Result<Self> {
        let db_name = to_db_name(&DatabaseDef::Index(def.clone())).wrap_err()?;
        
        let IndexDef(_coll, path, kind, key) = def;
        
        let db_opts = match (kind, key) {
            (IndexKind::Unique, KeyType::Int) => DatabaseOptions::create_map::<Unaligned<i64>>(),
            (IndexKind::Unique, KeyType::Float) => DatabaseOptions::create_map::<Unaligned<F64>>(),
            (IndexKind::Unique, KeyType::String) => DatabaseOptions::create_map::<str>(),
            (IndexKind::Unique, KeyType::Binary) => DatabaseOptions::create_map::<[u8]>(),
            (IndexKind::Unique, KeyType::Bool) => DatabaseOptions::create_map::<u8>(),
            (IndexKind::Duplicate, KeyType::Int) => DatabaseOptions::create_multimap::<Unaligned<i64>, Unaligned<Primary>>(),
            (IndexKind::Duplicate, KeyType::Float) => DatabaseOptions::create_multimap::<Unaligned<F64>, Unaligned<Primary>>(),
            (IndexKind::Duplicate, KeyType::String) => DatabaseOptions::create_multimap::<str, Unaligned<Primary>>(),
            (IndexKind::Duplicate, KeyType::Binary) => DatabaseOptions::create_multimap::<[u8], Unaligned<Primary>>(),
            (IndexKind::Duplicate, KeyType::Bool) => DatabaseOptions::create_multimap::<u8, Unaligned<Primary>>(),
        };
        
        let db = Arc::new(Database::open(env, Some(&db_name), &db_opts).wrap_err()?);
        
        Ok(Self { path, kind, key, db })
    }

    pub fn update_index(&self, access: &mut WriteAccessor, old_doc: Option<&Document>, new_doc: Option<&Document>) -> Result<()> {
        let doc = old_doc.or_else(|| new_doc).ok_or_else(|| "Either old_doc or new_doc or both must present").wrap_err()?;
        let id = doc.req_id()?;
        
        let old_keys = old_doc.map(|doc| self.extract(doc)).unwrap_or(HashSet::new());
        let new_keys = new_doc.map(|doc| self.extract(doc)).unwrap_or(HashSet::new());

        let (old_keys, new_keys) = (old_keys.difference(&new_keys), new_keys.difference(&old_keys));

        for key in old_keys {
            access.del_item(&self.db, key.into_raw(), &Unaligned::new(id)).wrap_err()?;
        }
        
        let f = match self.kind {
            IndexKind::Unique => NOOVERWRITE,
            IndexKind::Duplicate => NODUPDATA,
        };

        for key in new_keys {
            access.put(&self.db, key.into_raw(), &Unaligned::new(id), f)
                  .wrap_err()?;
        }
        
        Ok(())
    }

    fn extract(&self, doc: &Document) -> HashSet<KeyData> {
        let mut keys = HashSet::new();
        let path = self.path.split('.');
        extract_field_values(doc.get_data(), self.key, &path, &mut keys);
        keys
    }

    pub fn query_set<'a, I: Iterator<Item = &'a KeyData>>(&self, txn: &ReadTransaction, access: &ConstAccessor, keys: I) -> Result<HashSet<Primary>> {
        let mut out = HashSet::new();
        
        for key in keys {
            if let Some(key) = key.into_type(self.key) {
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

    pub fn query_range(&self, txn: &ReadTransaction, access: &ConstAccessor, beg: Option<&KeyData>, end: Option<&KeyData>) -> Result<HashSet<Primary>> {
        let mut out = HashSet::new();

        let beg = beg.and_then(|key| key.into_type(self.key));
        let end = end.and_then(|key| key.into_type(self.key));
        let cursor = txn.cursor(self.db.clone()).wrap_err()?;
        
        match (beg, end) {
            (Some(beg), None) => query_greaterequal(self.kind, access, cursor, beg, &mut out),
            (None, Some(end)) => query_lessequal(self.kind, access, cursor, end, &mut out),
            (Some(beg), Some(end)) => query_bounded(self.kind, access, cursor, beg, end, &mut out),
            _ => query_unbounded(self.kind, access, cursor, &mut out),
        }?;
        
        Ok(out)
    }

    pub fn query_iter(&self, txn: Arc<ReadTransaction<'static>>, order: OrderKind) -> Result<IndexIterator> {
        IndexIterator::new(txn, self.db.clone(), order)
    }
}

fn extract_field_values<'a, 'i: 'a, I: Iterator<Item = &'i str> + Clone>(doc: &'a Value, typ: KeyType, path: &'a I, keys: &mut HashSet<KeyData>) {
    let mut sub_path = path.clone();
    if let Some(name) = sub_path.next() {
        use serde_cbor::Value::*;
        match doc {
            Array(val) => val.iter().for_each(|doc| extract_field_values(doc, typ, path, keys)),
            Object(val) => if let Some(doc) = val.get(&name.to_owned().into()) {
                extract_field_values(doc, typ, &sub_path, keys);
            },
            _ => (),
        }
    } else {
        extract_field_primitives(doc, typ, keys);
    }
}

fn extract_field_primitives(doc: &Value, typ: KeyType, keys: &mut HashSet<KeyData>) {
    use serde_cbor::Value::*;
    match (typ, doc) {
        (_, Array(val)) => val.iter().for_each(|doc| extract_field_primitives(doc, typ, keys)),
        (typ, val) => {
            if let Some(val) = KeyData::from_val(&val) {
                if let Some(val) = val.into_type(typ) {
                    keys.insert(val.into_owned());
                }
            }
        },
    }
}

fn query_unbounded(kind: IndexKind, access: &ConstAccessor, cursor: Cursor, out: &mut HashSet<Primary>) -> Result<()> {
    match kind {
        IndexKind::Unique => {
            for item in CursorIter::new(
                MaybeOwned::Owned(cursor), access,
                |c, a| c.first(a),
                Cursor::next::<[u8], Unaligned<Primary>>)
                .wrap_err()?
            {
                match item {
                    Ok((_key, id)) => { out.insert(id.get()); },
                    Err(e) => return Err(e).wrap_err(),
                }
            }
        },
        IndexKind::Duplicate => {
            for item in CursorIter::new(
                MaybeOwned::Owned(cursor), access,
                |c, a| {
                    c.first::<[u8], [u8]>(a)?;
                    c.get_multiple::<[Unaligned<Primary>]>(a)
                }, |c, a| {
                    if let Some(ids) = c.next_multiple(a).to_opt()? {
                        Ok(ids)
                    } else {
                        c.next::<[u8], [u8]>(a)?;
                        c.get_multiple(a)
                    }
                })
                .wrap_err()?
            {
                match item {
                    Ok(ids) => { for id in ids { out.insert(id.get()); } },
                    Err(e) => return Err(e).wrap_err(),
                }
            }
        },
    }

    Ok(())
}

fn query_greaterequal(kind: IndexKind, access: &ConstAccessor, cursor: Cursor, key: Cow<'_, KeyData>, out: &mut HashSet<Primary>) -> Result<()> {
    match kind {
        IndexKind::Unique => {
            for item in CursorIter::new(
                MaybeOwned::Owned(cursor), access,
                |c, a| c.seek_range_k(a, key.into_raw()),
                Cursor::next::<[u8], Unaligned<Primary>>)
                .wrap_err()?
            {
                match item {
                    Ok((_key, id)) => { out.insert(id.get()); },
                    Err(e) => return Err(e).wrap_err(),
                }
            }
        },
        IndexKind::Duplicate => {
            for item in CursorIter::new(
                MaybeOwned::Owned(cursor), access,
                |c, a| {
                    c.seek_range_k::<[u8], [u8]>(a, key.into_raw())?;
                    c.get_multiple::<[Unaligned<Primary>]>(a)
                }, |c, a| {
                    if let Some(ids) = c.next_multiple(a).to_opt()? {
                        Ok(ids)
                    } else {
                        c.next::<[u8], [u8]>(a)?;
                        c.get_multiple(a)
                    }
                })
                .wrap_err()?
            {
                match item {
                    Ok(ids) => { for id in ids { out.insert(id.get()); } },
                    Err(e) => return Err(e).wrap_err(),
                }
            }
        },
    }

    Ok(())
}

fn query_lessequal(kind: IndexKind, access: &ConstAccessor, cursor: Cursor, key: Cow<'_, KeyData>, out: &mut HashSet<Primary>) -> Result<()> {
    match kind {
        IndexKind::Unique => {
            for item in CursorIter::new(
                MaybeOwned::Owned(cursor), access,
                |c, a| c.seek_range_k_prev(a, key.into_raw()),
                Cursor::prev::<[u8], Unaligned<Primary>>)
                .wrap_err()?
            {
                match item {
                    Ok((_key, id)) => { out.insert(id.get()); },
                    Err(e) => return Err(e).wrap_err(),
                }
            }
        },
        IndexKind::Duplicate => {
            for item in CursorIter::new(
                MaybeOwned::Owned(cursor), access,
                |c, a| {
                    c.seek_range_k_prev::<[u8], [u8]>(a, key.into_raw())?;
                    c.first_dup::<Unaligned<Primary>>(a)?;
                    c.get_multiple::<[Unaligned<Primary>]>(a)
                }, |c, a| {
                    if let Some(ids) = c.next_multiple(a).to_opt()? {
                        Ok(ids)
                    } else {
                        c.prev_nodup::<[u8], [u8]>(a)?;
                        c.first_dup::<Unaligned<Primary>>(a)?;
                        c.get_multiple(a)
                    }
                })
                .wrap_err()?
            {
                match item {
                    Ok(ids) => { for id in ids { out.insert(id.get()); } },
                    Err(e) => return Err(e).wrap_err(),
                }
            }
        },
    }

    Ok(())
}

fn query_bounded(kind: IndexKind, access: &ConstAccessor, cursor: Cursor, beg: Cow<'_, KeyData>, end: Cow<'_, KeyData>, out: &mut HashSet<Primary>) -> Result<()> {
    match kind {
        IndexKind::Unique => {
            for item in CursorIter::new(
                MaybeOwned::Owned(cursor), access,
                |c, a| c.seek_range_k(a, beg.into_raw()),
                Cursor::next::<[u8], Unaligned<Primary>>)
                .wrap_err()?
            {
                match item {
                    Ok((key, id)) => if &KeyData::from_raw(end.get_type(), key)? > &end {
                        break;
                    } else {
                        out.insert(id.get());
                    },
                    Err(e) => return Err(e).wrap_err(),
                }
            }
        },
        IndexKind::Duplicate => {
            for item in CursorIter::new(
                MaybeOwned::Owned(cursor), access,
                |c, a| {
                    let (key, _val) = c.seek_range_k::<[u8], [u8]>(a, beg.into_raw())?;
                    c.get_multiple::<[Unaligned<Primary>]>(a).map(|val| (key, val))
                }, |c, a| {
                    if let Some(ids) = c.next_multiple(a).to_opt()? {
                        c.get_current::<[u8], [u8]>(a).map(|(key, _val)| (key, ids))
                    } else {
                        let (key, _val) = c.next::<[u8], Unaligned<Primary>>(a)?;
                        c.get_multiple(a).map(|val| (key, val))
                    }
                })
                .wrap_err()?
            {
                match item {
                    Ok((key, ids)) =>  if &KeyData::from_raw(end.get_type(), key)? > &end {
                        break;
                    } else {
                        for id in ids { out.insert(id.get()); }
                    },
                    Err(e) => return Err(e).wrap_err(),
                }
            }
        },
    }

    Ok(())
}

pub struct IndexIterator {
    txn: Arc<ReadTransaction<'static>>,
    cur: Cursor<'static, 'static>,
    order: OrderKind,
    init: bool,
}

impl IndexIterator {
    pub fn new(txn: Arc<ReadTransaction<'static>>, db: Arc<Database<'static>>, order: OrderKind) -> Result<Self> {
        let cur = txn.cursor(db)?;

        Ok(Self { txn, cur, order, init: false })
    }
}

impl Iterator for IndexIterator {
    type Item = Result<Primary>;

    fn next(&mut self) -> Option<Self::Item> {
        let access = self.txn.access();
        match if self.init {
            match self.order {
                OrderKind::Asc => self.cur.next::<[u8], Unaligned<Primary>>(&access),
                OrderKind::Desc => self.cur.prev::<[u8], Unaligned<Primary>>(&access),
            }
        } else {
            self.init = true;
            match self.order {
                OrderKind::Asc => self.cur.first::<[u8], Unaligned<Primary>>(&access),
                OrderKind::Desc => self.cur.last::<[u8], Unaligned<Primary>>(&access),
            }
        }.to_opt() {
            Ok(Some((_key, id))) => Some(Ok(id.get())),
            Ok(None) => None,
            Err(e) => Some(Err(e).wrap_err()),
        }
    }
}
