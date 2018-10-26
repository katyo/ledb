use std::ops::{Deref, DerefMut};

use serde::{de::DeserializeOwned, Serialize};
use serde_cbor;

use super::{Document, Primary, Result, ResultWrap};
pub use serde_cbor::{ObjectKey, Value};

/// Raw document with id representation
#[derive(Debug, Clone, PartialEq)]
pub struct RawDocument(Option<Primary>, Value);

impl RawDocument {
    /// Create document using raw data
    #[inline]
    pub fn new(doc: Value) -> Self {
        RawDocument(None, doc)
    }

    /// Add id to document
    #[inline]
    pub fn with_id(mut self, id: Primary) -> Self {
        self.0 = Some(id);
        self
    }

    /// Remove id from document
    #[inline]
    pub fn without_id(mut self) -> Self {
        self.0 = None;
        self
    }

    /// Checks when document has primary key/identifier
    #[inline]
    pub fn has_id(&self) -> bool {
        self.0.is_some()
    }

    /// Get the primary key/identifier of document
    #[inline]
    pub fn get_id(&self) -> &Option<Primary> {
        &self.0
    }

    /// Require the primary key/identifier of document
    #[inline]
    pub fn req_id(&self) -> Result<Primary> {
        self.get_id()
            .ok_or_else(|| "Missing document id")
            .wrap_err()
    }

    /// Unwrap document value
    #[inline]
    pub fn into_inner(self) -> Value {
        self.1
    }

    /// Convert document to binary representation
    ///
    /// At this moment we use [CBOR](https://cbor.io/) for effectively store documents into DB backend.
    /// Since the internal representation does not contains primary identifier, it adds on reading documents from DB.
    ///
    pub fn into_bin(&self) -> Result<Vec<u8>> {
        serde_cbor::to_vec(&self.1).wrap_err()
    }

    /// Restore document from binary representation
    ///
    /// At this moment we use [CBOR](https://cbor.io/) for effectively store documents into DB backend.
    /// Since the internal representation does not contains primary identifier, it adds on reading documents from DB.
    ///
    pub fn from_bin(raw: &[u8]) -> Result<Self> {
        serde_cbor::from_slice(raw).map(Self::new).wrap_err()
    }

    /// Convert typed document to raw representation
    ///
    /// Typically the application deals with typed documents which represented by specific structures.
    /// The database backend processes generic document representation which is CBOR Value.
    pub fn from_doc<T>(doc: &T) -> Result<Self>
    where
        T: Serialize + Document + Sized,
    {
        let mut raw = to_value(doc)?;

        let id = if let Value::Object(ref mut obj) = &mut raw {
            // split primary field value
            obj.remove(&ObjectKey::String(T::primary_field().as_ref().into()))
        } else {
            return Err("Document must be represented as an object").wrap_err();
        };

        let id = match id {
            None => None,
            Some(Value::Null) => None,
            Some(Value::U64(id)) => Some(id as u32),
            Some(Value::I64(id)) => Some(id as u32),
            _ => return Err("Document primary must be an integer").wrap_err(),
        };

        Ok(RawDocument(id, raw))
    }

    /// Restore typed document from raw representation
    ///
    /// Typically the application deals with typed documents which represented by specific structures.
    /// The database backend processes generic document representation which is CBOR Value.
    pub fn into_doc<T>(self) -> Result<T>
    where
        T: DeserializeOwned + Document,
    {
        let RawDocument(id, mut raw) = self;
        if let Value::Object(ref mut obj) = &mut raw {
            if let Some(id) = &id {
                obj.insert(
                    ObjectKey::String(T::primary_field().as_ref().into()),
                    Value::U64(u64::from(*id)),
                );
            }
        } else {
            return Err("Document must be represented as an object").wrap_err();
        }

        serde_cbor::from_value(raw).wrap_err()
    }
}

impl Deref for RawDocument {
    type Target = Value;

    fn deref(&self) -> &Self::Target {
        &self.1
    }
}

impl DerefMut for RawDocument {
    fn deref_mut(&mut self) -> &mut Value {
        &mut self.1
    }
}

#[inline]
pub fn to_value<T: Serialize>(value: T) -> Result<Value> {
    serde_cbor::to_value(value).wrap_err()
}

#[cfg(test)]
mod test {
    use super::super::{Document, Identifier, Primary, RawDocument};

    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
    pub struct User {
        id: Option<Primary>,
        name: String,
        email: Option<String>,
    }

    impl Document for User {
        fn primary_field() -> Identifier {
            "id".into()
        }
    }

    #[test]
    fn raw_doc() {
        let src = User {
            id: None,
            name: "elen".into(),
            email: None,
        };
        let raw = RawDocument::from_doc(&src).unwrap();

        let res = raw.clone().into_doc::<User>().unwrap();
        assert_eq!(res, src);
        assert_eq!(RawDocument::from_doc(&res).unwrap(), raw);
    }

    #[test]
    fn gen_doc() {
        let src = User {
            id: None,
            name: "elen".into(),
            email: None,
        };
        let bin = RawDocument::from_doc(&src).unwrap().into_bin().unwrap();

        let raw = RawDocument::from_bin(&bin).unwrap();
        let res = RawDocument::from_doc(&src).unwrap();
        let doc = res.clone().into_doc::<User>().unwrap();

        assert_eq!(res, raw);
        assert_eq!(doc, src);
        assert_eq!(res.into_bin().unwrap(), raw.into_bin().unwrap());
    }

    #[test]
    #[ignore]
    fn duplicate_id() {
        let src = User {
            id: Some(1),
            name: "ivan".into(),
            email: None,
        };
        let res = RawDocument::from_doc(&src).unwrap().into_doc::<User>();

        assert!(res.is_err());
    }
}
