use bytes::{Bytes, IntoBuf};
use lmdb::traits::{AsLmdbBytes};
use serde::{Serialize, de::DeserializeOwned};
use serde_cbor;

pub struct RawVal(pub Bytes);

impl AsLmdbBytes for RawVal {
    fn as_lmdb_bytes(&self) -> &[u8] {
        self.0.as_ref()
    }
}

pub trait IntoVal {
    fn into_val(&self) -> RawVal;
}

pub trait FromVal {
    fn from_val(s: RawVal) -> Self;
}

impl<T: Serialize> IntoVal for T {
    fn into_val(&self) -> RawVal {
        RawVal(Bytes::from(serde_cbor::to_vec(self).unwrap()))
    }
}

impl<T: DeserializeOwned> FromVal for T {
    fn from_val(s: RawVal) -> Self {
        serde_cbor::from_slice(s.0.as_ref()).unwrap()
    }
}
