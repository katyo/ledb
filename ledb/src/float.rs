use ordered_float::OrderedFloat;
use lmdb::traits::{LmdbRawIfUnaligned, LmdbOrdKeyIfUnaligned};

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct F64(pub OrderedFloat<f64>);

unsafe impl LmdbRawIfUnaligned for F64 {
    fn reported_type() -> String { "F64".into() }
}

unsafe impl LmdbOrdKeyIfUnaligned for F64 { }
