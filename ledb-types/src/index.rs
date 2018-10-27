use std::borrow::Cow;
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet, VecDeque};
use std::iter::IntoIterator;
use std::ops::{Deref, DerefMut};
use std::rc::{Rc, Weak as RcWeak};
use std::sync::{Arc, Mutex, RwLock, Weak as ArcWeak};
use std::vec::IntoIter as VecIntoIter;

#[cfg(feature = "bytes")]
use bytes::{Bytes, BytesMut};

/// Indexed field definition
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KeyField {
    pub path: String,
    pub key: KeyType,
    pub kind: IndexKind,
}

impl KeyField {
    /// Create key field from field name
    #[inline]
    pub fn new<S: ToString>(path: S) -> Self {
        Self {
            path: path.to_string(),
            key: KeyType::default(),
            kind: IndexKind::default(),
        }
    }

    /// Add key type
    #[inline]
    pub fn with_type(mut self, key: KeyType) -> Self {
        self.key = key;
        self
    }

    /// Add index kind
    #[inline]
    pub fn with_kind(mut self, kind: IndexKind) -> Self {
        self.kind = kind;
        self
    }

    /// Set parent path
    ///
    /// This makes key field to be child for parent path
    #[inline]
    pub fn set_parent<S: AsRef<str>>(&mut self, parent: S) {
        self.path.insert_str(0, ".");
        self.path.insert_str(0, parent.as_ref());
    }

    /// Add parent path
    ///
    /// This makes key field to be child for parent path
    #[inline]
    pub fn with_parent<S: AsRef<str>>(mut self, parent: S) -> Self {
        self.set_parent(parent);
        self
    }
}

impl<S: ToString> From<(S,)> for KeyField {
    fn from((path,): (S,)) -> Self {
        Self::new(path)
    }
}

impl<'a, S: ToString> From<&'a (S,)> for KeyField {
    fn from((path,): &(S,)) -> Self {
        Self::new(path.to_string())
    }
}

impl<S: ToString> From<(S, KeyType)> for KeyField {
    fn from((path, key): (S, KeyType)) -> Self {
        Self::new(path).with_type(key)
    }
}

impl<'a, S: ToString> From<&'a (S, KeyType)> for KeyField {
    fn from((path, key): &(S, KeyType)) -> Self {
        Self::new(path.to_string()).with_type(*key)
    }
}

impl<S: ToString> From<(S, IndexKind)> for KeyField {
    fn from((path, kind): (S, IndexKind)) -> Self {
        Self::new(path).with_kind(kind)
    }
}

impl<'a, S: ToString> From<&'a (S, IndexKind)> for KeyField {
    fn from((path, kind): &(S, IndexKind)) -> Self {
        Self::new(path.to_string()).with_kind(*kind)
    }
}

impl<S: ToString> From<(S, KeyType, IndexKind)> for KeyField {
    fn from((path, key, kind): (S, KeyType, IndexKind)) -> Self {
        Self {
            path: path.to_string(),
            key,
            kind,
        }
    }
}

impl<'a, S: ToString> From<&'a (S, KeyType, IndexKind)> for KeyField {
    fn from((path, key, kind): &(S, KeyType, IndexKind)) -> Self {
        Self {
            path: path.to_string(),
            key: *key,
            kind: *kind,
        }
    }
}

impl<S: ToString> From<(S, IndexKind, KeyType)> for KeyField {
    fn from((path, kind, key): (S, IndexKind, KeyType)) -> Self {
        Self {
            path: path.to_string(),
            key,
            kind,
        }
    }
}

impl<'a, S: ToString> From<&'a (S, IndexKind, KeyType)> for KeyField {
    fn from((path, kind, key): &(S, IndexKind, KeyType)) -> Self {
        Self {
            path: path.to_string(),
            key: *key,
            kind: *kind,
        }
    }
}

impl Into<(String, KeyType, IndexKind)> for KeyField {
    fn into(self) -> (String, KeyType, IndexKind) {
        let KeyField { path, key, kind } = self;
        (path, key, kind)
    }
}

impl Into<(String, IndexKind, KeyType)> for KeyField {
    fn into(self) -> (String, IndexKind, KeyType) {
        let KeyField { path, key, kind } = self;
        (path, kind, key)
    }
}

/// Indexed fields definition
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KeyFields(Vec<KeyField>);

impl KeyFields {
    /// Create new key fields set
    #[inline]
    pub fn new() -> Self {
        KeyFields(Vec::new())
    }

    /// Add key field to set
    #[inline]
    pub fn with_field<T>(mut self, field: T) -> Self
    where
        KeyField: From<T>,
    {
        self.push(KeyField::from(field));
        self
    }

    /// Add key fields to set
    #[inline]
    pub fn with_fields(mut self, mut fields: KeyFields) -> Self {
        self.append(&mut *fields);
        self
    }

    /// Set parent path
    ///
    /// This makes key fields in set to be children for parent path
    pub fn set_parent<S: AsRef<str>>(&mut self, parent: S) {
        for ref mut field in &mut self.0 {
            field.set_parent(&parent);
        }
    }

    /// Add parent path
    ///
    /// This makes key fields in set to be children for parent path
    pub fn with_parent<S: AsRef<str>>(mut self, parent: S) -> Self {
        self.set_parent(&parent);
        self
    }
}

/*
impl From<Vec<KeyField>> for KeyFields {
    fn from(vec: Vec<KeyField>) -> Self {
        KeyFields(vec)
    }
}

impl<'a> From<&'a [KeyField]> for KeyFields {
    fn from(arr: &[KeyField]) -> Self {
        KeyFields(arr.into())
    }
}
 */

impl<'a, T> From<&'a [T]> for KeyFields
where
    T: Clone,
    KeyField: From<T>,
{
    fn from(arr: &[T]) -> Self {
        KeyFields(arr.iter().cloned().map(KeyField::from).collect())
    }
}

impl<T> From<Vec<T>> for KeyFields
where
    KeyField: From<T>,
{
    fn from(vec: Vec<T>) -> Self {
        KeyFields(vec.into_iter().map(KeyField::from).collect())
    }
}

impl AsRef<[KeyField]> for KeyFields {
    fn as_ref(&self) -> &[KeyField] {
        self.0.as_ref()
    }
}

impl AsMut<[KeyField]> for KeyFields {
    fn as_mut(&mut self) -> &mut [KeyField] {
        self.0.as_mut()
    }
}

impl Deref for KeyFields {
    type Target = Vec<KeyField>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for KeyFields {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl IntoIterator for KeyFields {
    type Item = KeyField;
    type IntoIter = VecIntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

/// The type of key
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum KeyType {
    #[serde(rename = "int")]
    Int,
    #[serde(rename = "float")]
    Float,
    #[serde(rename = "string")]
    String,
    #[serde(rename = "binary")]
    Binary,
    #[serde(rename = "bool")]
    Bool,
}

impl Default for KeyType {
    fn default() -> Self {
        KeyType::Binary
    }
}

/// The kind of index
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum IndexKind {
    /// Index which may contains duplicates
    #[serde(rename = "index")]
    Index,
    /// Index which contains unique keys only
    #[serde(rename = "unique")]
    Unique,
}

impl Default for IndexKind {
    fn default() -> Self {
        IndexKind::Index
    }
}

/// Field key type inference
pub trait DocumentKeyType {
    /// Get type of field key by field type
    fn key_type() -> KeyType {
        KeyType::default()
    }
}

impl DocumentKeyType for bool {
    fn key_type() -> KeyType {
        KeyType::Bool
    }
}

impl DocumentKeyType for u8 {
    fn key_type() -> KeyType {
        KeyType::Int
    }
}

impl DocumentKeyType for i8 {
    fn key_type() -> KeyType {
        KeyType::Int
    }
}

impl DocumentKeyType for u16 {
    fn key_type() -> KeyType {
        KeyType::Int
    }
}

impl DocumentKeyType for i16 {
    fn key_type() -> KeyType {
        KeyType::Int
    }
}

impl DocumentKeyType for u32 {
    fn key_type() -> KeyType {
        KeyType::Int
    }
}

impl DocumentKeyType for i32 {
    fn key_type() -> KeyType {
        KeyType::Int
    }
}

impl DocumentKeyType for u64 {
    fn key_type() -> KeyType {
        KeyType::Int
    }
}

impl DocumentKeyType for i64 {
    fn key_type() -> KeyType {
        KeyType::Int
    }
}

impl DocumentKeyType for f32 {
    fn key_type() -> KeyType {
        KeyType::Float
    }
}

impl DocumentKeyType for f64 {
    fn key_type() -> KeyType {
        KeyType::Float
    }
}

impl DocumentKeyType for String {
    fn key_type() -> KeyType {
        KeyType::String
    }
}

#[cfg(feature = "bytes")]
impl DocumentKeyType for Bytes {
    fn key_type() -> KeyType {
        KeyType::Binary
    }
}

#[cfg(feature = "bytes")]
impl DocumentKeyType for BytesMut {
    fn key_type() -> KeyType {
        KeyType::Binary
    }
}

impl<'a, T: DocumentKeyType> DocumentKeyType for &'a T {
    fn key_type() -> KeyType {
        T::key_type()
    }
}

impl<'a, T: DocumentKeyType> DocumentKeyType for &'a mut T {
    fn key_type() -> KeyType {
        T::key_type()
    }
}

impl<T: DocumentKeyType> DocumentKeyType for Box<T> {
    fn key_type() -> KeyType {
        T::key_type()
    }
}

impl<T: DocumentKeyType> DocumentKeyType for Rc<T> {
    fn key_type() -> KeyType {
        T::key_type()
    }
}

impl<T: DocumentKeyType> DocumentKeyType for RcWeak<T> {
    fn key_type() -> KeyType {
        T::key_type()
    }
}

impl<T: DocumentKeyType> DocumentKeyType for Arc<T> {
    fn key_type() -> KeyType {
        T::key_type()
    }
}

impl<T: DocumentKeyType> DocumentKeyType for ArcWeak<T> {
    fn key_type() -> KeyType {
        T::key_type()
    }
}

impl<T: DocumentKeyType> DocumentKeyType for Mutex<T> {
    fn key_type() -> KeyType {
        T::key_type()
    }
}

impl<T: DocumentKeyType> DocumentKeyType for RwLock<T> {
    fn key_type() -> KeyType {
        T::key_type()
    }
}

impl<'a, T: DocumentKeyType> DocumentKeyType for &'a [T] {
    fn key_type() -> KeyType {
        T::key_type()
    }
}

impl<'a, T: DocumentKeyType> DocumentKeyType for &'a mut [T] {
    fn key_type() -> KeyType {
        T::key_type()
    }
}

impl<'a, T: DocumentKeyType + Clone> DocumentKeyType for Cow<'a, T> {
    fn key_type() -> KeyType {
        T::key_type()
    }
}

impl<T: DocumentKeyType> DocumentKeyType for [T] {
    fn key_type() -> KeyType {
        T::key_type()
    }
}

impl<T: DocumentKeyType> DocumentKeyType for Vec<T> {
    fn key_type() -> KeyType {
        T::key_type()
    }
}

impl<T: DocumentKeyType> DocumentKeyType for VecDeque<T> {
    fn key_type() -> KeyType {
        T::key_type()
    }
}

impl<T: DocumentKeyType> DocumentKeyType for HashSet<T> {
    fn key_type() -> KeyType {
        T::key_type()
    }
}

impl<K, T: DocumentKeyType> DocumentKeyType for HashMap<K, T> {
    fn key_type() -> KeyType {
        T::key_type()
    }
}

impl<T: DocumentKeyType> DocumentKeyType for BTreeSet<T> {
    fn key_type() -> KeyType {
        T::key_type()
    }
}

impl<K, T: DocumentKeyType> DocumentKeyType for BTreeMap<K, T> {
    fn key_type() -> KeyType {
        T::key_type()
    }
}

impl<T: DocumentKeyType> DocumentKeyType for Option<T> {
    fn key_type() -> KeyType {
        T::key_type()
    }
}
