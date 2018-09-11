use std::mem::transmute;
use std::cmp::Ordering;
use lmdb::traits::{LmdbRawIfUnaligned, LmdbOrdKeyIfUnaligned};

#[derive(Clone, Copy)]
pub struct F64(pub f64);

impl Eq for F64 {}

impl PartialEq for F64 {
    fn eq(&self, other: &F64) -> bool {
        self.0 == other.0
    }
}

impl Ord for F64 {
    fn cmp(&self, other: &F64) -> Ordering {
        let mut a = unsafe { transmute::<f64, i64>(self.0) };
        let mut b = unsafe { transmute::<f64, i64>(other.0) };
        if a < 0 { a ^= 0x7fff_ffff_ffff_ffff; }
        if b < 0 { b ^= 0x7fff_ffff_ffff_ffff; }
        a.cmp(&b)
    }
}

impl PartialOrd for F64 {
    fn partial_cmp(&self, other: &F64) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

unsafe impl LmdbRawIfUnaligned for F64 {
    fn reported_type() -> String { "F64".into() }
}

unsafe impl LmdbOrdKeyIfUnaligned for F64 { }
