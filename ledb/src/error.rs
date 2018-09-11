use std::fmt::Display;
use std::str::Utf8Error;
use std::io::Error as IoError;
use std::sync::PoisonError;
use std::result::Result as StdResult;
use serde_cbor::error::Error as CborError;
use ron::{ser::Error as RonEncError, de::Error as RonDecError};
use lmdb::error::Error as DbError;

#[derive(Debug)]
pub enum Error {
    DocError(String),
    DbError(DbError),
    StrError(Utf8Error),
    DataError(CborError),
    StorageError(String),
    IoError(IoError),
    SyncError(String),
}

pub type Result<T> = StdResult<T, Error>;

impl From<CborError> for Error {
    fn from(e: CborError) -> Self {
        Error::DataError(e)
    }
}

impl From<RonDecError> for Error {
    fn from(e: RonDecError) -> Self {
        Error::StorageError(format!("{}", e))
    }
}

impl From<RonEncError> for Error {
    fn from(e: RonEncError) -> Self {
        Error::StorageError(format!("{}", e))
    }
}

impl From<DbError> for Error {
    fn from(e: DbError) -> Self {
        Error::DbError(e)
    }
}

impl From<IoError> for Error {
    fn from(e: IoError) -> Self {
        Error::IoError(e)
    }
}

impl<E> From<PoisonError<E>> for Error
    where PoisonError<E>: Display
{
    fn from(e: PoisonError<E>) -> Self {
        Error::SyncError(format!("{}", e))
    }
}

impl From<Utf8Error> for Error {
    fn from(e: Utf8Error) -> Self {
        Error::StrError(e)
    }
}

impl From<String> for Error {
    fn from(e: String) -> Self {
        Error::DocError(e)
    }
}

impl<'a> From<&'a str> for Error {
    fn from(e: &'a str) -> Self {
        Error::DocError(e.into())
    }
}

pub trait ResultWrap<T> {
    fn wrap_err(self) -> Result<T>;
}

impl<T, E> ResultWrap<T> for StdResult<T, E>
    where Error: From<E>
{
    fn wrap_err(self) -> Result<T> {
        self.map_err(Error::from)
    }
}

/*
impl<T> ResultWrap for StdResult<T, ron::de::Error> {
    type Result = Result<T>;
    fn wrap_err(self) -> Self::Result {
        self.map_err(|e| Error::DocError(format!("{}", e)))
    }
}

impl<T> ResultWrap for StdResult<T, ron::ser::Error> {
    type Result = Result<T>;
    fn wrap_err(self) -> Self::Result {
        self.map_err(|e| Error::DocError(format!("{}", e)))
    }
}

impl<T> ResultWrap for StdResult<T, DbError> {
    type Result = Result<T>;
    fn wrap_err(self) -> Self::Result {
        self.map_err(Error::DbError)
    }
}

impl<T> ResultWrap for StdResult<T, IoError> {
    type Result = Result<T>;
    fn wrap_err(self) -> Self::Result {
        self.map_err(|e| Error::IoError(e))
    }
}

impl<T, E> ResultWrap for StdResult<T, PoisonError<E>>
    where PoisonError<E>: Display
{
    type Result = Result<T>;
    fn wrap_err(self) -> Self::Result {
        self.map_err(|e| Error::SyncError(format!("{}", e)))
    }
}

impl<T> ResultWrap for StdResult<T, String> {
    type Result = Result<T>;
    fn wrap_err(self) -> Self::Result {
        self.map_err(|e| Error::SyncError(e))
    }
}
*/
