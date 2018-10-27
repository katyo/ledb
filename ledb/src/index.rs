use lmdb::{
    put::{NODUPDATA, NOOVERWRITE},
    traits::CreateCursor,
    ConstAccessor, Cursor, CursorIter, Database, DatabaseOptions, LmdbResultExt, MaybeOwned,
    ReadTransaction, Unaligned, WriteAccessor,
};
use ron::ser::to_string as to_db_name;
use serde_cbor::ObjectKey;
use std::collections::HashSet;
use std::mem::replace;
use std::ops::Deref;
use std::sync::atomic::{AtomicBool, Ordering as AtomicOrdering};
use std::sync::Arc;
use supercow::{ext::ConstDeref, Supercow};

use super::{
    DatabaseDef, Enumerable, IndexKind, KeyData, KeyField, KeyType, OrderKind, Primary,
    RawDocument, Result, ResultWrap, Serial, Storage, Value,
};
use float::F64;

/// The definition of index
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct IndexDef(
    /// Unique serial
    pub Serial,
    /// Collection name
    pub String,
    /// Field path
    pub String,
    pub IndexKind,
    pub KeyType,
);

impl IndexDef {
    pub fn new<C: Into<String>, P: Into<String>>(
        coll: C,
        path: P,
        kind: IndexKind,
        key: KeyType,
    ) -> Self {
        IndexDef(0, coll.into(), path.into(), kind, key)
    }
}

impl Enumerable for IndexDef {
    fn enumerate(&mut self, serial: Serial) {
        self.0 = serial;
    }
}

struct IndexData {
    path: String,
    kind: IndexKind,
    key: KeyType,
    db: Database<'static>,
    // Remove marker
    delete: AtomicBool,
}

/// Index for document field
#[derive(Clone)]
pub(crate) struct Index(Option<Arc<IndexData>>);

impl Index {
    pub(crate) fn new(storage: Storage, def: IndexDef) -> Result<Self> {
        let db_name = to_db_name(&DatabaseDef::Index(def.clone())).wrap_err()?;

        let IndexDef(_serial, _coll, path, kind, key) = def;

        let db_opts = match (kind, key) {
            (IndexKind::Unique, KeyType::Int) => DatabaseOptions::create_map::<Unaligned<i64>>(),
            (IndexKind::Unique, KeyType::Float) => DatabaseOptions::create_map::<Unaligned<F64>>(),
            (IndexKind::Unique, KeyType::String) => DatabaseOptions::create_map::<str>(),
            (IndexKind::Unique, KeyType::Binary) => DatabaseOptions::create_map::<[u8]>(),
            (IndexKind::Unique, KeyType::Bool) => DatabaseOptions::create_map::<u8>(),
            (IndexKind::Duplicate, KeyType::Int) => {
                DatabaseOptions::create_multimap::<Unaligned<i64>, Unaligned<Primary>>()
            }
            (IndexKind::Duplicate, KeyType::Float) => {
                DatabaseOptions::create_multimap::<Unaligned<F64>, Unaligned<Primary>>()
            }
            (IndexKind::Duplicate, KeyType::String) => {
                DatabaseOptions::create_multimap::<str, Unaligned<Primary>>()
            }
            (IndexKind::Duplicate, KeyType::Binary) => {
                DatabaseOptions::create_multimap::<[u8], Unaligned<Primary>>()
            }
            (IndexKind::Duplicate, KeyType::Bool) => {
                DatabaseOptions::create_multimap::<u8, Unaligned<Primary>>()
            }
        };

        let db = Database::open(storage, Some(&db_name), &db_opts).wrap_err()?;

        Ok(Index(Some(Arc::new(IndexData {
            path,
            kind,
            key,
            db,
            delete: AtomicBool::new(false),
        }))))
    }

    fn handle(&self) -> &IndexData {
        if let Some(handle) = &self.0 {
            handle
        } else {
            unreachable!();
        }
    }

    pub fn path(&self) -> &str {
        &self.handle().path
    }

    pub fn kind(&self) -> IndexKind {
        self.handle().kind
    }

    pub fn key(&self) -> KeyType {
        self.handle().key
    }

    pub fn field(&self) -> KeyField {
        let handle = self.handle();

        KeyField::new(handle.path.clone())
            .with_type(handle.key)
            .with_kind(handle.kind)
    }

    pub(crate) fn update_index(
        &self,
        access: &mut WriteAccessor,
        old_doc: Option<&RawDocument>,
        new_doc: Option<&RawDocument>,
    ) -> Result<()> {
        let doc = old_doc
            .or_else(|| new_doc)
            .ok_or_else(|| "Either old_doc or new_doc or both must present")
            .wrap_err()?;
        let id = doc.req_id()?;

        let old_keys = old_doc
            .map(|doc| self.extract(doc))
            .unwrap_or(HashSet::new());
        let new_keys = new_doc
            .map(|doc| self.extract(doc))
            .unwrap_or(HashSet::new());

        let (old_keys, new_keys) = (
            old_keys.difference(&new_keys),
            new_keys.difference(&old_keys),
        );

        let handle = self.handle();

        //println!("Update index {} --{:?} ++{:?}", &handle.path, &old_keys, &new_keys);

        for key in old_keys {
            access
                .del_item(&handle.db, key.into_raw(), &Unaligned::new(id))
                .wrap_err()?;
        }

        let f = match handle.kind {
            IndexKind::Unique => NOOVERWRITE,
            IndexKind::Duplicate => NODUPDATA,
        };

        for key in new_keys {
            access
                .put(&handle.db, key.into_raw(), &Unaligned::new(id), f)
                .wrap_err()?;
        }

        Ok(())
    }

    fn extract(&self, doc: &RawDocument) -> HashSet<KeyData> {
        let mut keys = HashSet::new();
        let handle = self.handle();
        let path = handle.path.split('.');
        extract_field_values(&*doc, handle.key, &path, &mut keys);
        keys
    }

    pub(crate) fn query_set<'a, I: Iterator<Item = &'a KeyData>>(
        &self,
        txn: &ReadTransaction,
        access: &ConstAccessor,
        keys: I,
    ) -> Result<HashSet<Primary>> {
        let mut out = HashSet::new();
        let handle = self.handle();

        for key in keys {
            if let Some(key) = key.into_type(handle.key) {
                let mut cursor = txn.cursor(self.clone()).wrap_err()?;

                match handle.kind {
                    IndexKind::Unique => match cursor
                        .seek_k_both::<[u8], Unaligned<Primary>>(&access, key.into_raw())
                        .to_opt()
                    {
                        Ok(Some((_key, id))) => {
                            out.insert(id.get());
                        }
                        Err(e) => return Err(e).wrap_err(),
                        _ => (),
                    },
                    IndexKind::Duplicate => {
                        match cursor
                            .seek_k::<[u8], Unaligned<Primary>>(&access, key.into_raw())
                            .to_opt()
                        {
                            Ok(Some(..)) => (),
                            Ok(None) => continue,
                            Err(e) => return Err(e).wrap_err(),
                        }

                        for res in CursorIter::new(
                            MaybeOwned::Owned(cursor),
                            &access,
                            |c, a| c.get_multiple::<[Unaligned<Primary>]>(&a),
                            Cursor::next_multiple::<[Unaligned<Primary>]>,
                        ).wrap_err()?
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

    pub(crate) fn query_range(
        &self,
        txn: &ReadTransaction,
        access: &ConstAccessor,
        beg: Option<(&KeyData, bool)>,
        end: Option<(&KeyData, bool)>,
    ) -> Result<HashSet<Primary>> {
        let mut out = HashSet::new();
        let handle = self.handle();

        let beg = beg.and_then(|(key, inc)| key.into_type(handle.key).map(|key| (key, inc)));
        let end = end.and_then(|(key, inc)| key.into_type(handle.key).map(|key| (key, inc)));
        let cursor = txn.cursor(self.clone()).wrap_err()?;

        match handle.kind {
            IndexKind::Unique => {
                for item in CursorIter::new(
                    MaybeOwned::Owned(cursor),
                    access,
                    |c, a| match beg {
                        Some((beg_key, beg_inc)) => {
                            let p = c.seek_range_k(a, beg_key.into_raw())?;
                            if beg_inc {
                                Ok(p)
                            } else {
                                c.next(a)
                            }
                        }
                        _ => c.first(a),
                    },
                    Cursor::next::<[u8], Unaligned<Primary>>,
                ).wrap_err()?
                {
                    match (item, &end) {
                        (Ok((key, id)), Some((end_key, end_inc))) => {
                            let key = KeyData::from_raw(end_key.get_type(), key)?;
                            if &key < &end_key || *end_inc && &key <= &end_key {
                                out.insert(id.get());
                            } else {
                                break;
                            }
                        }
                        (Ok((_, id)), _) => {
                            out.insert(id.get());
                        }
                        (Err(e), _) => return Err(e).wrap_err(),
                    }
                }
            }
            IndexKind::Duplicate => {
                for item in CursorIter::new(
                    MaybeOwned::Owned(cursor),
                    access,
                    |c, a| {
                        let key = match beg {
                            Some((beg_key, beg_inc)) => {
                                let p = c.seek_range_k::<[u8], [u8]>(a, beg_key.into_raw())?.0;
                                if beg_inc {
                                    p
                                } else {
                                    c.next::<[u8], [u8]>(a)?.0
                                }
                            }
                            _ => c.first::<[u8], [u8]>(a)?.0,
                        };
                        c.get_multiple::<[Unaligned<Primary>]>(a)
                            .map(|val| (key, val))
                    },
                    |c, a| {
                        if let Some(ids) = c.next_multiple(a).to_opt()? {
                            c.get_current::<[u8], [u8]>(a).map(|(key, _val)| (key, ids))
                        } else {
                            let key = c.next::<[u8], Unaligned<Primary>>(a)?.0;
                            c.get_multiple(a).map(|ids| (key, ids))
                        }
                    },
                ).wrap_err()?
                {
                    match (item, &end) {
                        (Ok((key, ids)), Some((end_key, end_inc))) => {
                            let key = KeyData::from_raw(end_key.get_type(), key)?;
                            if &key < &end_key || *end_inc && &key <= &end_key {
                                for id in ids {
                                    out.insert(id.get());
                                }
                            } else {
                                break;
                            }
                        }
                        (Ok((_, ids)), _) => {
                            for id in ids {
                                out.insert(id.get());
                            }
                        }
                        (Err(e), _) => return Err(e).wrap_err(),
                    }
                }
            }
        }

        Ok(out)
    }

    pub(crate) fn query_iter(
        &self,
        txn: Arc<ReadTransaction<'static>>,
        order: OrderKind,
    ) -> Result<IndexIterator> {
        IndexIterator::new(txn, self.clone(), order)
    }

    pub(crate) fn purge(&self, access: &mut WriteAccessor) -> Result<()> {
        let handle = self.handle();
        access.clear_db(&handle.db).wrap_err()
    }

    pub(crate) fn to_delete(&self, access: &mut WriteAccessor) -> Result<()> {
        self.purge(access)?;
        let handle = self.handle();
        handle.delete.store(true, AtomicOrdering::SeqCst);
        Ok(())
    }
}

impl Drop for Index {
    fn drop(&mut self) {
        let data = replace(&mut self.0, None).unwrap();

        if let Ok(IndexData { db, delete, .. }) = Arc::try_unwrap(data) {
            if delete.load(AtomicOrdering::SeqCst) {
                if let Err(e) = db.delete() {
                    eprintln!("Error when deleting index db: {}", e);
                }
            }
        }
    }
}

impl Deref for Index {
    type Target = Database<'static>;

    #[inline]
    fn deref(&self) -> &Self::Target {
        if let Some(data) = &self.0 {
            &data.db
        } else {
            unreachable!()
        }
    }
}

unsafe impl ConstDeref for Index {
    type Target = Database<'static>;

    #[inline]
    fn const_deref(&self) -> &Self::Target {
        if let Some(data) = &self.0 {
            &data.db
        } else {
            unreachable!()
        }
    }
}

impl<'a> Into<Supercow<'a, Database<'a>>> for Index {
    fn into(self) -> Supercow<'a, Database<'a>> {
        let this = self.clone();
        Supercow::shared(this)
    }
}

fn extract_field_values<'a, 'i: 'a, I: Iterator<Item = &'i str> + Clone>(
    doc: &'a Value,
    typ: KeyType,
    path: &'a I,
    keys: &mut HashSet<KeyData>,
) {
    let mut sub_path = path.clone();
    if let Some(name) = sub_path.next() {
        use Value::*;
        match doc {
            Array(val) => val
                .iter()
                .for_each(|doc| extract_field_values(doc, typ, path, keys)),
            Object(val) if name == "*" => val
                .iter()
                .for_each(|(_key, doc)| extract_field_values(doc, typ, path, keys)),
            Object(val) => if let Some(doc) = val.get(&name.to_owned().into()) {
                extract_field_values(doc, typ, &sub_path, keys);
            },
            _ => (),
        }
    } else {
        extract_field_primitives(doc, typ, keys);
    }
}

fn key_to_val(key: &ObjectKey) -> Value {
    match key {
        ObjectKey::Integer(val) => Value::I64(*val),
        ObjectKey::String(val) => Value::String(val.clone()),
        ObjectKey::Bool(val) => Value::Bool(*val),
        ObjectKey::Bytes(val) => Value::Bytes(val.clone()),
        ObjectKey::Null => Value::Null,
    }
}

fn extract_field_primitives(doc: &Value, typ: KeyType, keys: &mut HashSet<KeyData>) {
    use serde_cbor::Value::*;
    match (typ, doc) {
        (_, Array(val)) => val
            .iter()
            .for_each(|doc| extract_field_primitives(doc, typ, keys)),
        (_, Object(val)) => val
            .iter()
            .for_each(|(key, _doc)| extract_field_primitives(&key_to_val(key), typ, keys)),
        (typ, val) => {
            if let Some(val) = KeyData::from_val(&val) {
                if let Some(val) = val.into_type(typ) {
                    // prevent indexing empty values
                    if !val.is_empty() {
                        keys.insert(val.into_owned());
                    }
                }
            }
        }
    }
}

pub(crate) struct IndexIterator {
    txn: Arc<ReadTransaction<'static>>,
    cur: Cursor<'static, 'static>,
    order: OrderKind,
    init: bool,
}

impl IndexIterator {
    pub fn new(txn: Arc<ReadTransaction<'static>>, coll: Index, order: OrderKind) -> Result<Self> {
        let cur = txn.cursor(coll)?;

        Ok(Self {
            txn,
            cur,
            order,
            init: false,
        })
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
        }.to_opt()
        {
            Ok(Some((_key, id))) => Some(Ok(id.get())),
            Ok(None) => None,
            Err(e) => Some(Err(e).wrap_err()),
        }
    }
}
