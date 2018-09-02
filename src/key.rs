use lmdb::traits::{AsLmdbBytes};
use std::str::from_utf8;
use bytes::{Bytes, Buf, BytesMut, BufMut, IntoBuf};

pub struct RawKey(pub Bytes);

impl AsLmdbBytes for RawKey {
    fn as_lmdb_bytes(&self) -> &[u8] {
        self.0.as_ref()
    }
}

pub trait IntoKey {
    fn into_key(&self) -> RawKey;
}

pub trait FromKey {
    fn from_key(src: RawKey) -> Self;
}

type Num = u64;
const MID: Num = 1<<31;

impl IntoKey for u8 {
    fn into_key(&self) -> RawKey {
        let mut buf = BytesMut::with_capacity(8);
        buf.put_u64_be(Num::from(*self) + MID);
        RawKey(buf.freeze())
    }
}

impl FromKey for u8 {
    fn from_key(src: RawKey) -> Self {
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
    fn from_key(src: RawKey) -> Self {
        let mut buf = src.0.into_buf();
        let val = buf.get_u64_be();
        (val as i64 - MID as i64) as i8
    }
}

/*
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
    fn into_key(&self) -> RawKey {
        let mut buf = BytesMut::with_capacity(8);
        buf.put_u64_be((*self as i64 + MID as i64) as u64);
        RawKey(buf.freeze())
    }
}

impl FromKey for u32 {
    fn from_key(src: RawKey) -> Self {
        let mut buf = src.0.into_buf();
        let val = buf.get_u64_be();
        (val as i64 - MID as i64) as u32
    }
}

impl IntoKey for i32 {
    fn into_key(&self) -> RawKey {
        let mut buf = BytesMut::with_capacity(8);
        buf.put_u64_be((*self as i64 + MID as i64) as u64);
        RawKey(buf.freeze())
    }
}

impl FromKey for i32 {
    fn from_key(src: RawKey) -> Self {
        let mut buf = src.0.into_buf();
        let val = buf.get_u64_be();
        (val as i64 - MID as i64) as i32
    }
}

/*
impl IntoKey for u64 {
    fn into_key(&self) -> RawKey {
        let mut buf = BytesMut::with_capacity(8);
        buf.put_int_be(*self as Num + MID, 12);
        RawKey(buf.freeze())
    }
}

impl FromKey for u64 {
    fn from_key(src: RawKey) -> Self {
        let mut buf = src.0.into_buf();
        let val = buf.get_int_be(12);
        (val - MID) as u64
    }
}

impl IntoKey for i64 {
    fn into_key(&self) -> RawKey {
        let mut buf = BytesMut::with_capacity(8);
        buf.put_int_be(*self as Num + MID, 12);
        RawKey(buf.freeze())
    }
}

impl FromKey for i64 {
    fn from_key(src: RawKey) -> Self {
        let mut buf = src.0.into_buf();
        let val = buf.get_int_be(12);
        (val - MID)
    }
}
*/

impl IntoKey for String {
    fn into_key(&self) -> RawKey {
        RawKey(Bytes::from(self.as_str()))
    }
}

impl FromKey for String {
    fn from_key(src: RawKey) -> Self {
        String::from(from_utf8(src.0.into_buf().bytes()).unwrap())
    }
}
