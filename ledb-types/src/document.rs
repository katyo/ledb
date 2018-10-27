use super::{Identifier, KeyFields};
use std::borrow::Cow;
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet, VecDeque};
use std::rc::{Rc, Weak as RcWeak};
use std::sync::{Arc, Mutex, RwLock, Weak as ArcWeak};

/// Primary key (document identifier)
pub type Primary = u32;

/// Identified document representation
pub trait Document {
    /// Get the name of primary field
    fn primary_field() -> Identifier {
        "$".into()
    }

    /// Get other key fields (indexes)
    fn key_fields() -> KeyFields {
        KeyFields::new()
    }
}

impl<'a, T: Document> Document for &'a T {
    fn primary_field() -> Identifier {
        T::primary_field()
    }

    fn key_fields() -> KeyFields {
        T::key_fields()
    }
}

impl<'a, T: Document> Document for &'a mut T {
    fn primary_field() -> Identifier {
        T::primary_field()
    }

    fn key_fields() -> KeyFields {
        T::key_fields()
    }
}

impl<'a, T: Document> Document for &'a [T] {
    fn primary_field() -> Identifier {
        T::primary_field()
    }

    fn key_fields() -> KeyFields {
        T::key_fields()
    }
}

impl<'a, T: Document> Document for &'a mut [T] {
    fn primary_field() -> Identifier {
        T::primary_field()
    }

    fn key_fields() -> KeyFields {
        T::key_fields()
    }
}

impl<T: Document> Document for [T] {
    fn primary_field() -> Identifier {
        T::primary_field()
    }

    fn key_fields() -> KeyFields {
        T::key_fields()
    }
}

impl<T: Document> Document for Vec<T> {
    fn primary_field() -> Identifier {
        T::primary_field()
    }

    fn key_fields() -> KeyFields {
        T::key_fields()
    }
}

impl<T: Document> Document for VecDeque<T> {
    fn primary_field() -> Identifier {
        T::primary_field()
    }

    fn key_fields() -> KeyFields {
        T::key_fields()
    }
}

impl<T: Document> Document for HashSet<T> {
    fn primary_field() -> Identifier {
        T::primary_field()
    }

    fn key_fields() -> KeyFields {
        T::key_fields()
    }
}

impl<K, T: Document> Document for HashMap<K, T> {
    fn primary_field() -> Identifier {
        T::primary_field()
    }

    fn key_fields() -> KeyFields {
        T::key_fields()
    }
}

impl<T: Document> Document for BTreeSet<T> {
    fn primary_field() -> Identifier {
        T::primary_field()
    }

    fn key_fields() -> KeyFields {
        T::key_fields()
    }
}

impl<K, T: Document> Document for BTreeMap<K, T> {
    fn primary_field() -> Identifier {
        T::primary_field()
    }

    fn key_fields() -> KeyFields {
        T::key_fields()
    }
}

impl<'a, T: Document> Document for Box<T> {
    fn primary_field() -> Identifier {
        T::primary_field()
    }

    fn key_fields() -> KeyFields {
        T::key_fields()
    }
}

impl<'a, T: Document> Document for Rc<T> {
    fn primary_field() -> Identifier {
        T::primary_field()
    }

    fn key_fields() -> KeyFields {
        T::key_fields()
    }
}

impl<'a, T: Document> Document for RcWeak<T> {
    fn primary_field() -> Identifier {
        T::primary_field()
    }

    fn key_fields() -> KeyFields {
        T::key_fields()
    }
}

impl<'a, T: Document> Document for Arc<T> {
    fn primary_field() -> Identifier {
        T::primary_field()
    }

    fn key_fields() -> KeyFields {
        T::key_fields()
    }
}

impl<'a, T: Document> Document for ArcWeak<T> {
    fn primary_field() -> Identifier {
        T::primary_field()
    }

    fn key_fields() -> KeyFields {
        T::key_fields()
    }
}

impl<'a, T: Document> Document for Mutex<T> {
    fn primary_field() -> Identifier {
        T::primary_field()
    }

    fn key_fields() -> KeyFields {
        T::key_fields()
    }
}

impl<'a, T: Document> Document for RwLock<T> {
    fn primary_field() -> Identifier {
        T::primary_field()
    }

    fn key_fields() -> KeyFields {
        T::key_fields()
    }
}

impl<'a, T: Document + Clone> Document for Cow<'a, T> {
    fn primary_field() -> Identifier {
        T::primary_field()
    }

    fn key_fields() -> KeyFields {
        T::key_fields()
    }
}

impl<T: Document> Document for Option<T> {
    fn primary_field() -> Identifier {
        T::primary_field()
    }

    fn key_fields() -> KeyFields {
        T::key_fields()
    }
}

#[cfg(feature = "json")]
impl Document for serde_json::Value {}

#[cfg(feature = "cbor")]
impl Document for serde_cbor::Value {}
