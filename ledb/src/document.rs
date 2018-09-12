use std::ops::Deref;

use serde::{Serialize, de::DeserializeOwned};
use serde_cbor;

use super::{Result, ResultWrap};
pub use serde_cbor::{Value, ObjectKey};

pub type Primary = u32;

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
    #[inline]
    pub fn new(data: T) -> Self {
        Self { id: None, data }
    }

    #[inline]
    pub fn new_with_id(id: Primary, data: T) -> Self {
        Self { id: Some(id), data }
    }

    #[inline]
    pub fn get_id(&self) -> &Option<Primary> {
        &self.id
    }

    #[inline]
    pub fn req_id(&self) -> Result<Primary> {
        self.get_id().ok_or_else(|| "Missing document id").wrap_err()
    }

    #[inline]
    pub fn get_data(&self) -> &T {
        &self.data
    }

    #[inline]
    pub fn has_id(&self) -> bool {
        self.id != None
    }

    #[inline]
    pub fn set_id(&mut self, id: Primary) {
        self.id = Some(id);
    }

    #[inline]
    pub fn with_id(mut self, id: Primary) -> Self {
        self.set_id(id);
        self
    }

    #[inline]
    pub fn remove_id(&mut self) {
        self.id = None;
    }

    #[inline]
    pub fn without_id(mut self) -> Self {
        self.remove_id();
        self
    }

    #[inline]
    pub fn set_data(&mut self, data: T) {
        self.data = data;
    }

    #[inline]
    pub fn with_data(mut self, data: T) -> Self {
        self.set_data(data);
        self
    }

    pub fn into_raw(&self) -> Result<Vec<u8>> where T: Serialize {
        serde_cbor::to_vec(&self).wrap_err()
    }

    pub fn from_raw(raw: &[u8]) -> Result<Self> where T: DeserializeOwned {
        serde_cbor::from_slice(raw).wrap_err()
    }
    
    pub fn into_gen(&self) -> Result<Document<Value>> where T: Serialize {
        Ok(Document { id: self.id, data: serde_cbor::to_value(&self.data).wrap_err()? })
    }

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
    fn duplicate_id() {
        let src = Document::new(UserWithId { id: Some(1), name: "ivan".into(), email: None });
        let res = Document::<UserWithId>::from_raw(&src.into_raw().unwrap());

        assert!(res.is_err());
    }
}
