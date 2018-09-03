use std::fmt::Display;
use serde_cbor::{Value, ObjectKey};
use lmdb::error::Error as LmdbError;
use lmdbffi;

pub type Id = i64;
pub type Document = Value;
pub type Binary = Vec<u8>;
pub type Field = ObjectKey;

pub const NOT_FOUND: LmdbError = LmdbError::Code(lmdbffi::MDB_NOTFOUND);

pub fn document_field<S: AsRef<str>>(s: S) -> Field {
    ObjectKey::String(s.as_ref().into())
}

pub trait ResultWrap {
    type Result;
    fn wrap_err(self) -> Self::Result;
}

impl<T, E: Display> ResultWrap for Result<T, E> {
    type Result = Result<T, String>;
    fn wrap_err(self) -> Self::Result {
        self.map_err(|e| format!("{}", e))
    }
}
