use bytes::{Bytes, IntoBuf};
use serde::{Serialize, de::DeserializeOwned};
use serde_cbor;

pub trait IntoVal {
    fn into_val_bytes(&self) -> Bytes;
    fn into_val(&self) -> Vec<u8> {
        self.into_val_bytes().to_vec()
    }
}

pub trait FromVal: Sized {
    fn from_val_slice(s: &[u8]) -> Self;
    fn from_val<S: AsRef<[u8]>>(s: S) -> Self {
        Self::from_val_slice(s.as_ref())
    }
}

impl<T: Serialize> IntoVal for T {
    fn into_val(&self) -> Vec<u8> {
        serde_cbor::to_vec(self).unwrap()
    }
    fn into_val_bytes(&self) -> Bytes {
        Bytes::from(serde_cbor::to_vec(self).unwrap())
    }
}

impl<T: DeserializeOwned> FromVal for T {
    fn from_val_slice(s: &[u8]) -> Self {
        serde_cbor::from_slice(s.as_ref()).unwrap()
    }
}
