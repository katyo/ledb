use std::str::from_utf8;
use bytes::{Bytes, Buf, BytesMut, BufMut, IntoBuf};

pub trait IntoKey {
    fn into_key_bytes(&self) -> Bytes;
    fn into_key(&self) -> Vec<u8> {
        self.into_key_bytes().to_vec()
    }
}

pub trait FromKey: Sized {
    fn from_key_slice(s: &[u8]) -> Self;
    fn from_key<S: AsRef<[u8]>>(s: S) -> Self {
        Self::from_key_slice(s.as_ref())
    }
}

type Num = u64;
const MID: Num = 1<<31;

/*
impl IntoKey for u8 {
    fn into_key(&self) -> RawKey {
        let mut buf = BytesMut::with_capacity(8);
        buf.put_u64_be(Num::from(*self) + MID);
        RawKey(buf.freeze())
    }
}

impl FromKey for u8 {
    fn from_key(src: &RawKey) -> Self {
        let mut buf = src.0.into_buf();
        let val = buf.get_u64_be();
        (val - MID) as u8
    }
}

impl IntoKey for i8 {
    fn into_key(&self) -> RawKey {
        let mut buf = BytesMut::with_capacity(8);
        buf.put_u64_be((*self as i64 + MID as i64) as Num);
        RawKey(buf.freeze())
    }
}

impl FromKey for i8 {
    fn from_key(src: &RawKey) -> Self {
        let mut buf = src.0.into_buf();
        let val = buf.get_u64_be();
        (val as i64 - MID as i64) as i8
    }
}

impl IntoKey for u16 {
    fn into_key(&self) -> RawKey {
        let mut buf = BytesMut::with_capacity(8);
        buf.put_int_be((*self as Num) + MID, 12);
        RawKey(buf.freeze())
    }
}

impl FromKey for u16 {
    fn from_key(src: RawKey) -> Self {
        let mut buf = src.0.into_buf();
        let val = buf.get_int_be(12);
        (val - MID) as u16
    }
}

impl IntoKey for i16 {
    fn into_key(&self) -> RawKey {
        let mut buf = BytesMut::with_capacity(8);
        buf.put_int_be((*self as Num) + MID, 12);
        RawKey(buf.freeze())
    }
}

impl FromKey for i16 {
    fn from_key(src: RawKey) -> Self {
        let mut buf = src.0.into_buf();
        let val = buf.get_int_be(12);
        (val - MID) as i16
    }
}
*/

impl IntoKey for u32 {
    fn into_key_bytes(&self) -> Bytes {
        let mut buf = BytesMut::with_capacity(8);
        buf.put_u64_be((*self as i64 + MID as i64) as u64);
        buf.freeze()
    }
}

impl FromKey for u32 {
    fn from_key_slice(src: &[u8]) -> Self {
        let mut buf = src.into_buf();
        let val = buf.get_u64_be();
        (val as i64 - MID as i64) as u32
    }
}

impl IntoKey for i32 {
    fn into_key_bytes(&self) -> Bytes {
        let mut buf = BytesMut::with_capacity(8);
        buf.put_u64_be((*self as i64 + MID as i64) as u64);
        buf.freeze()
    }
}

impl FromKey for i32 {
    fn from_key_slice(src: &[u8]) -> Self {
        let mut buf = src.into_buf();
        let val = buf.get_u64_be();
        (val as i64 - MID as i64) as i32
    }
}

impl IntoKey for u64 {
    fn into_key_bytes(&self) -> Bytes {
        let mut buf = BytesMut::with_capacity(8);
        buf.put_u64_be((*self as i64 + MID as i64) as u64);
        buf.freeze()
    }
}

impl FromKey for u64 {
    fn from_key_slice(src: &[u8]) -> Self {
        let mut buf = src.into_buf();
        let val = buf.get_u64_be();
        (val as i64 - MID as i64) as u64
    }
}

impl IntoKey for i64 {
    fn into_key_bytes(&self) -> Bytes {
        let mut buf = BytesMut::with_capacity(8);
        buf.put_u64_be((*self as i64 + MID as i64) as u64);
        buf.freeze()
    }
}

impl FromKey for i64 {
    fn from_key_slice(src: &[u8]) -> Self {
        let mut buf = src.into_buf();
        let val = buf.get_u64_be();
        (val as i64 - MID as i64)
    }
}

impl IntoKey for String {
    fn into_key_bytes(&self) -> Bytes {
        Bytes::from(self.as_str())
    }
}

impl FromKey for String {
    fn from_key_slice(src: &[u8]) -> Self {
        String::from(from_utf8(src.into_buf().bytes()).unwrap())
    }
}

impl IntoKey for [u8] {
    fn into_key_bytes(&self) -> Bytes {
        Bytes::from(self)
    }
}

impl IntoKey for Vec<u8> {
    fn into_key_bytes(&self) -> Bytes {
        Bytes::from(self.as_slice())
    }
}

impl FromKey for Vec<u8> {
    fn from_key_slice(src: &[u8]) -> Self {
        Vec::from(src)
    }
}

impl IntoKey for Bytes {
    fn into_key_bytes(&self) -> Bytes {
        self.clone()
    }
}

impl FromKey for Bytes {
    fn from_key_slice(src: &[u8]) -> Self {
        Bytes::from(src)
    }
}

impl IntoKey for bool {
    fn into_key_bytes(&self) -> Bytes {
        (if *self { 1u32 } else { 0u32 }).into_key_bytes()
    }
}

impl FromKey for bool {
    fn from_key_slice(src: &[u8]) -> Self {
        if u32::from_key_slice(src) == 1u32 { true } else { false }
    }
}
