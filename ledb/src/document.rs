use std::ops::Deref;
use std::borrow::Borrow;
use std::hash::{Hash, Hasher};

use serde::{Serialize, de::DeserializeOwned};
use serde_cbor;

use super::{Result, ResultWrap};
pub use serde_cbor::{Value, ObjectKey};

/// Generic string indentifier
///
/// This type is used as name for collections and fields
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Identifier {
    Owned(String),
    Refer(&'static str),
}

impl Default for Identifier {
    fn default() -> Self {
        Identifier::Refer("")
    }
}

impl Eq for Identifier {}

impl PartialEq for Identifier {
    fn eq(&self, other: &Self) -> bool {
        self.as_ref() == other.as_ref()
    }
}

impl Hash for Identifier {
    fn hash<H: Hasher>(&self, state: &mut H) {
        use self::Identifier::*;
        match self {
            Owned(s) => s.hash(state),
            Refer(s) => s.hash(state),
        }
    }
}

impl AsRef<str> for Identifier {
    fn as_ref(&self) -> &str {
        use self::Identifier::*;
        match self {
            Owned(s) => &s,
            Refer(s) => s,
        }
    }
}

impl Borrow<str> for Identifier {
    #[inline]
    fn borrow(&self) -> &str {
        self.as_ref()
    }
}

impl From<&'static str> for Identifier {
    fn from(s: &'static str) -> Self {
        Identifier::Refer(s)
    }
}

impl From<String> for Identifier {
    fn from(s: String) -> Self {
        Identifier::Owned(s)
    }
}

impl<'a> From<&'a String> for Identifier {
    fn from(s: &String) -> Self {
        Identifier::Owned(s.clone())
    }
}

/// Primary key (document identifier)
pub type Primary = u32;

/// Identified document representation
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Document<T = Value> {
    #[serde(rename="$")]
    id: Option<Primary>,
    #[serde(flatten)]
    data: T,
}

impl<T> Deref for Document<T> {
    type Target = T;

    fn deref(&self) -> &T {
        &self.data
    }
}

impl<T> Document<T> {
    /// Create document using serializable data
    #[inline]
    pub fn new(data: T) -> Self {
        Self { id: None, data }
    }

    /// Create identified document using serializable data
    #[inline]
    pub fn new_with_id(id: Primary, data: T) -> Self {
        Self { id: Some(id), data }
    }

    /// Get the primary key/identifier of document
    #[inline]
    pub fn get_id(&self) -> &Option<Primary> {
        &self.id
    }

    /// Require the primary key/identifier of document
    #[inline]
    pub fn req_id(&self) -> Result<Primary> {
        self.get_id().ok_or_else(|| "Missing document id").wrap_err()
    }

    /// Get document contents (document data without identifier)
    #[inline]
    pub fn get_data(&self) -> &T {
        &self.data
    }

    /// Checks when document has primary key/identifier
    #[inline]
    pub fn has_id(&self) -> bool {
        self.id != None
    }

    /// Set primary key/identifier to document
    #[inline]
    pub fn set_id(&mut self, id: Primary) {
        self.id = Some(id);
    }

    /// Add primary key/identifier to document
    #[inline]
    pub fn with_id(mut self, id: Primary) -> Self {
        self.set_id(id);
        self
    }

    /// Remove primary key/identifier from document
    #[inline]
    pub fn remove_id(&mut self) {
        self.id = None;
    }

    /// Clear primary key/identifier from document
    #[inline]
    pub fn without_id(mut self) -> Self {
        self.remove_id();
        self
    }

    /// Set document contents
    #[inline]
    pub fn set_data(&mut self, data: T) {
        self.data = data;
    }

    /// Add new contents to document
    #[inline]
    pub fn with_data(mut self, data: T) -> Self {
        self.set_data(data);
        self
    }

    /// Convert document to binary representation
    ///
    /// At this moment we use [CBOR](https://cbor.io/) for effectively store documents into DB backend.
    /// Since the internal representation does not contains primary identifier, it adds on reading documents from DB.
    ///
    pub fn into_raw(&self) -> Result<Vec<u8>> where T: Serialize {
        serde_cbor::to_vec(&self.data).wrap_err()
    }

    /// Restore document from binary representation
    ///
    /// At this moment we use [CBOR](https://cbor.io/) for effectively store documents into DB backend.
    /// Since the internal representation does not contains primary identifier, it adds on reading documents from DB.
    ///
    pub fn from_raw(raw: &[u8]) -> Result<Self> where T: DeserializeOwned {
        serde_cbor::from_slice(raw).wrap_err()
    }

    /// Convert typed document to generic representation
    ///
    /// Typically the application deals with typed documents which represented by specific structures.
    /// The database backend processes generic document representation which is CBOR Value.
    pub fn into_gen(&self) -> Result<Document<Value>> where T: Serialize {
        Ok(Document { id: self.id, data: serde_cbor::to_value(&self.data).wrap_err()? })
    }

    /// Restore typed document from generic representation
    ///
    /// Typically the application deals with typed documents which represented by specific structures.
    /// The database backend processes generic document representation which is CBOR Value.
    pub fn from_gen(gen: Document<Value>) -> Result<Document<T>> where T: DeserializeOwned {
        Ok(Document { id: gen.id, data: serde_cbor::from_value(gen.data).wrap_err()? })
    }
}

impl Document<Value> {
    #[inline]
    pub fn from_doc<T>(doc: &Document<T>) -> Result<Self> where T: Serialize {
        doc.into_gen()
    }

    #[inline]
    pub fn into_doc<T>(self) -> Result<Document<T>> where T: DeserializeOwned {
        Document::from_gen(self)
    }
}

pub fn to_value<T: Serialize>(value: T) -> Result<Value> {
    serde_cbor::to_value(value).wrap_err()
}

#[cfg(test)]
mod test {
    use super::{Primary, Document};

    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
    pub struct User {
        name: String,
        email: Option<String>,
    }

    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
    pub struct UserWithId {
        #[serde(rename="$")]
        id: Option<Primary>,
        name: String,
        email: Option<String>,
    }
    
    #[test]
    fn raw_doc() {
        let src = Document::new(User { name: "elen".into(), email: None });
        let raw = src.into_raw().unwrap();
        
        let res = Document::<User>::from_raw(&raw).unwrap();
        assert_eq!(res, src);
        assert_eq!(res.into_raw().unwrap(), raw);
    }

    #[test]
    fn gen_doc() {
        let src = Document::new(User { name: "elen".into(), email: None });
        let raw = src.into_raw().unwrap();
        
        let gen = Document::from_raw(&raw).unwrap();
        let res = src.into_gen().unwrap();
        let doc = Document::<User>::from_gen(res.clone()).unwrap();
        
        assert_eq!(res, gen);
        assert_eq!(doc, src);
        assert_eq!(res.into_raw().unwrap(), gen.into_raw().unwrap());
    }

    #[test]
    #[ignore]
    fn duplicate_id() {
        let src = Document::new(UserWithId { id: Some(1), name: "ivan".into(), email: None });
        let res = Document::<UserWithId>::from_raw(&src.into_raw().unwrap());

        assert!(res.is_err());
    }
}
