use std::fmt::{self, Display};
use std::str::Utf8Error;
use std::io::Error as IoError;
use std::sync::PoisonError;
use std::result::Result as StdResult;
use serde_cbor::error::Error as CborError;
use ron::{ser::Error as RonEncError, de::Error as RonDecError};
use lmdb::error::Error as DbError;

/// Database error type
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

impl Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use Error::*;
        match self {
            DocError(s) => write!(f, "Document error: {}", s),
            DbError(e) => write!(f, "Database error: {}", e),
            StrError(e) => write!(f, "String error: {}", e),
            DataError(e) => write!(f, "Data coding error: {}", e),
            StorageError(s) => write!(f, "Storage error: {}", s),
            IoError(e) => write!(f, "I/O Error: {}", e),
            SyncError(s) => write!(f, "Sync error: {}", s),
        }
    }
}

/// Database result type
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

/// The helper for converting results with different error types into generic result
pub trait ResultWrap<T> {
    fn wrap_err(self) -> Result<T>;
}

impl<T, E> ResultWrap<T> for StdResult<T, E>
    where Error: From<E>
{
    /// Convert result
    fn wrap_err(self) -> Result<T> {
        self.map_err(Error::from)
    }
}
