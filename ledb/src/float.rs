use lmdb::traits::{LmdbOrdKeyIfUnaligned, LmdbRawIfUnaligned};
use ordered_float::OrderedFloat;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct F64(pub OrderedFloat<f64>);

unsafe impl LmdbRawIfUnaligned for F64 {
    fn reported_type() -> String {
        f64::reported_type()
    }
}

unsafe impl LmdbOrdKeyIfUnaligned for F64 {}
