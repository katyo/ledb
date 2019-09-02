use ledb::{Value};

pub fn refine(value: Value) -> Value {
    use self::Value::*;

    match value {
        Integer(n) => Float(n as f64),
        Array(v) => Array(v.into_iter().map(refine).collect()),
        Map(h) => Map(h.into_iter().map(|(k, v)| (refine(k), refine(v))).collect()),
        _ => value,
    }
}
