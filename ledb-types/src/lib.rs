extern crate serde;

#[macro_use]
extern crate serde_derive;

#[cfg(feature = "json")]
extern crate serde_json;

#[cfg(feature = "cbor")]
extern crate serde_cbor;

use std::borrow::Borrow;
use std::borrow::Cow;
use std::hash::{Hash, Hasher};
use std::ops::Deref;
use std::rc::{Rc, Weak as RcWeak};
use std::sync::{Arc, Weak as ArcWeak};

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

impl Deref for Identifier {
    type Target = str;

    fn deref(&self) -> &str {
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
pub trait Document {
    fn primary_field() -> Identifier {
        "$".into()
    }
}

impl<'a, T: Document> Document for &'a T {
    fn primary_field() -> Identifier {
        T::primary_field()
    }
}

impl<'a, T: Document> Document for &'a mut T {
    fn primary_field() -> Identifier {
        T::primary_field()
    }
}

impl<T: Document> Document for [T] {
    fn primary_field() -> Identifier {
        T::primary_field()
    }
}

impl<T: Document> Document for Vec<T> {
    fn primary_field() -> Identifier {
        T::primary_field()
    }
}

impl<'a, T: Document> Document for Box<T> {
    fn primary_field() -> Identifier {
        T::primary_field()
    }
}

impl<'a, T: Document> Document for Rc<T> {
    fn primary_field() -> Identifier {
        T::primary_field()
    }
}

impl<'a, T: Document> Document for RcWeak<T> {
    fn primary_field() -> Identifier {
        T::primary_field()
    }
}

impl<'a, T: Document> Document for Arc<T> {
    fn primary_field() -> Identifier {
        T::primary_field()
    }
}

impl<'a, T: Document> Document for ArcWeak<T> {
    fn primary_field() -> Identifier {
        T::primary_field()
    }
}

impl<'a, T: Document + Clone> Document for Cow<'a, T> {
    fn primary_field() -> Identifier {
        T::primary_field()
    }
}

#[cfg(feature = "json")]
impl Document for serde_json::Value {}

#[cfg(feature = "cbor")]
impl Document for serde_cbor::Value {}
