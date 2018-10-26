use super::Identifier;
use std::borrow::Cow;
use std::rc::{Rc, Weak as RcWeak};
use std::sync::{Arc, Weak as ArcWeak};

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
