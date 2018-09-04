use std::sync::{Arc};
use std::iter::once;
use std::str::from_utf8;
use std::mem::transmute;
use byteorder::{ByteOrder, NativeEndian};
use ron::ser::to_string as to_db_name;
use lmdb::{Environment, put::Flags as PutFlags, Database, DatabaseOptions, ReadTransaction, WriteTransaction, Unaligned};

use types::{Id, Document, document_field, ResultWrap, NOT_FOUND};
use storage::{DatabaseDef};

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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum IndexType {
    #[serde(rename="uint")]
    UInt,
    #[serde(rename="sint")]
    Int,
    #[serde(rename="str")]
    String,
    #[serde(rename="raw")]
    Binary,
    #[serde(rename="bool")]
    Bool,
}

impl Default for IndexType {
    fn default() -> Self { IndexType::Binary }
}

pub enum IndexData {
    UInt(u64),
    Int(i64),
    String(String),
    Binary(Vec<u8>),
    Bool(bool),
}

impl IndexData {
    pub fn from_raw(typ: &IndexType, raw: &[u8]) -> Result<Self, String> {
        use self::IndexData::*;
        Ok(match typ {
            IndexType::UInt => {
                if raw.len() != 8 { return Err("UInt index must be 8 bytes length".into()) }
                UInt(NativeEndian::read_u64(raw))
            },
            IndexType::Int => {
                if raw.len() != 8 { return Err("Int index must be 8 bytes length".into()) }
                Int(NativeEndian::read_i64(raw))
            },
            IndexType::String => String(from_utf8(raw).wrap_err()?.into()),
            IndexType::Binary => Binary(Vec::from(raw)),
            IndexType::Bool => {
                if raw.len() != 1 { return Err("Bool index must be 1 byte length".into()) }
                Bool(if raw[0] == 0 { false } else { true })
            },
        })
    }
    
    pub fn into_raw(&self) -> &[u8] {
        use self::IndexData::*;
        match self {
            UInt(val) => unsafe { transmute::<&u64, &[u8;8]>(val) },
            Int(val) => unsafe { transmute::<&i64, &[u8;8]>(val) },
            String(val) => val.as_bytes(),
            Binary(val) => val.as_slice(),
            Bool(val) => unsafe { transmute::<&bool, &[u8;1]>(val) },
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IndexDef (
    /// Collection name
    pub String,
    /// Field path
    pub String,
    pub IndexKind,
    pub IndexType,
);

pub struct Index {
    pub(crate) path: String,
    pub(crate) kind: IndexKind,
    pub(crate) key: IndexType,
    pub(crate) db: Arc<Database<'static>>,
}

impl Index {
    pub fn new(env: Arc<Environment>, def: IndexDef) -> Result<Self, String> {
        let db_name = to_db_name(&DatabaseDef::Index(def.clone())).wrap_err()?;
        
        let IndexDef(_coll, path, kind, key) = def;

        let db_opts = match (kind, key) {
            (IndexKind::Unique, IndexType::UInt) => DatabaseOptions::create_map::<Unaligned<u64>>(),
            (IndexKind::Unique, IndexType::Int) => DatabaseOptions::create_map::<Unaligned<i64>>(),
            (IndexKind::Unique, IndexType::String) => DatabaseOptions::create_map::<str>(),
            (IndexKind::Unique, IndexType::Binary) => DatabaseOptions::create_map::<[u8]>(),
            (IndexKind::Unique, IndexType::Bool) => DatabaseOptions::create_map::<u8>(),
            (IndexKind::Duplicate, IndexType::UInt) => DatabaseOptions::create_multimap::<Unaligned<u64>, Unaligned<Id>>(),
            (IndexKind::Duplicate, IndexType::Int) => DatabaseOptions::create_multimap::<Unaligned<i64>, Unaligned<Id>>(),
            (IndexKind::Duplicate, IndexType::String) => DatabaseOptions::create_multimap::<str, Unaligned<Id>>(),
            (IndexKind::Duplicate, IndexType::Binary) => DatabaseOptions::create_multimap::<[u8], Unaligned<Id>>(),
            (IndexKind::Duplicate, IndexType::Bool) => DatabaseOptions::create_multimap::<u8, Unaligned<Id>>(),
        };
        
        let db = Arc::new(Database::open(
            env.clone(), Some(&db_name), &db_opts)
                          .wrap_err()?);
        
        Ok(Self { path, kind, key, db })
    }
    
    pub fn add(&self, txn: &mut WriteTransaction, data: &Vec<(Id, Vec<IndexData>)>) -> Result<(), String> {
        if data.len() == 0 {
            return Ok(());
        }
        
        let txn = txn.child_tx().wrap_err()?;

        {
            let mut access = txn.access();
            let f = PutFlags::empty();
            
            for (key, vals) in data {
                for val in vals {
                    access.put(&self.db, val.into_raw(), &Unaligned::new(*key), f)
                      .wrap_err()?;
                }
            }
        }

        txn.commit().wrap_err()?;
        
        Ok(())
    }

    //pub fn del(&self, 

    pub fn extract(&self, doc: &Document) -> Vec<IndexData> {
        let mut keys = Vec::new();
        let path = self.path.split('.');
        extract_field_values(doc, &self.key, &path, &mut keys);
        keys
    }
}

fn extract_field_values<'a, 'i: 'a, I: Iterator<Item = &'i str> + Clone>(doc: &'a Document, typ: &IndexType, path: &'a I, keys: &mut Vec<IndexData>) {
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

fn extract_field_primitives(doc: &Document, typ: &IndexType, keys: &mut Vec<IndexData>) {
    use serde_cbor::Value::*;
    match (typ, doc) {
        (IndexType::UInt, U64(val)) => keys.push(IndexData::UInt(*val)),
        (IndexType::Int, I64(val)) => keys.push(IndexData::Int(*val)),
        (IndexType::Binary, Bytes(val)) => keys.push(IndexData::Binary(val.clone())),
        (IndexType::String, String(val)) => keys.push(IndexData::String(val.clone())),
        (IndexType::Bool, Bool(val)) => keys.push(IndexData::Bool(*val)),
        (_, Array(val)) => val.iter().for_each(|doc| extract_field_primitives(doc, typ, keys)),
        _ => (),
    }
}
