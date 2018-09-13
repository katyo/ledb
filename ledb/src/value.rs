use std::str::from_utf8;
use std::mem::transmute;
use std::borrow::Cow;
use byteorder::{ByteOrder, NativeEndian};
use ordered_float::OrderedFloat;

use super::{Result, ResultWrap, Value};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum KeyType {
    #[serde(rename="int")]
    Int,
    #[serde(rename="flt")]
    Float,
    #[serde(rename="str")]
    String,
    #[serde(rename="raw")]
    Binary,
    #[serde(rename="bool")]
    Bool,
}

impl Default for KeyType {
    fn default() -> Self { KeyType::Binary }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(untagged)]
pub enum KeyData {
    Int(i64),
    #[serde(with="float")]
    Float(OrderedFloat<f64>),
    String(String),
    Binary(Vec<u8>),
    Bool(bool),
}

mod float {
    use super::{OrderedFloat};
    use serde::{Serializer, Deserializer, Deserialize};
    
    pub fn serialize<S: Serializer>(OrderedFloat(val): &OrderedFloat<f64>, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_f64(*val)
    }
    
    pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<OrderedFloat<f64>, D::Error> {
        f64::deserialize(deserializer).map(OrderedFloat)
    }
}

impl KeyData {
    pub fn from_raw(typ: KeyType, raw: &[u8]) -> Result<Self> {
        use self::KeyData::*;
        Ok(match typ {
            KeyType::Int => {
                if raw.len() != 8 { return Err("Int key must be 8 bytes length".into()) }
                Int(NativeEndian::read_i64(raw))
            },
            KeyType::Float => {
                if raw.len() != 8 { return Err("Float key must be 8 bytes length".into()) }
                Float(OrderedFloat(NativeEndian::read_f64(raw)))
            },
            KeyType::String => String(from_utf8(raw).wrap_err()?.into()),
            KeyType::Binary => Binary(Vec::from(raw)),
            KeyType::Bool => {
                if raw.len() != 1 { return Err("Bool key must be 1 byte length".into()) }
                Bool(if raw[0] == 0 { false } else { true })
            },
        })
    }
    
    pub fn into_raw(&self) -> &[u8] {
        use self::KeyData::*;
        match self {
            Int(val) => unsafe { transmute::<&i64, &[u8;8]>(val) },
            Float(val) => unsafe { transmute::<&f64, &[u8;8]>(val) },
            String(val) => val.as_bytes(),
            Binary(val) => val.as_slice(),
            Bool(val) => unsafe { transmute::<&bool, &[u8;1]>(val) },
        }
    }

    pub fn from_val(val: &Value) -> Option<Self> {
        use serde_cbor::Value::*;
        Some(match val {
            U64(val) => KeyData::Int(*val as i64),
            I64(val) => KeyData::Int(*val),
            F64(val) => KeyData::Float(OrderedFloat(*val)),
            Bytes(val) => KeyData::Binary(val.clone()),
            String(val) => KeyData::String(val.clone()),
            Bool(val) => KeyData::Bool(*val),
            _ => return None,
        })
    }

    pub fn as_type<'a>(&'a self, typ: KeyType) -> Option<&'a KeyData> {
        use self::KeyData::*;
        Some(match (typ, self) {
            (KeyType::Int, Int(..)) |
            (KeyType::Float, Float(..)) |
            (KeyType::Binary, Binary(..)) |
            (KeyType::String, String(..)) |
            (KeyType::Bool, Bool(..)) => self,
            _ => return None,
        })
    }

    pub fn into_type<'a>(&'a self, typ: KeyType) -> Option<Cow<'a, KeyData>> {
        use self::KeyData::*;
        Some(if let Some(v) = self.as_type(typ) {
            Cow::Borrowed(v)
        } else {
            Cow::Owned(match (typ, self) {
                (KeyType::Float, Int(v)) => Float(OrderedFloat(*v as f64)),
                (KeyType::Int, Float(v)) => Int(v.round() as i64),
                (KeyType::String, Int(v)) => String(v.to_string()),
                (KeyType::String, Float(v)) => String(v.to_string()),
                (KeyType::String, Bool(v)) => String(v.to_string()),
                (KeyType::Int, String(v)) => Int(if let Ok(v) = v.parse() { v } else { return None }),
                (KeyType::Float, String(v)) => Float(if let Ok(v) = v.parse() { OrderedFloat(v) } else { return None }),
                (KeyType::Bool, String(v)) => Bool(if let Ok(v) = v.parse() { v } else { return None }),
                _ => return None,
            })
        })
    }

    pub fn get_type(&self) -> KeyType {
        use self::KeyData::*;
        match self {
            Int(..) => KeyType::Int,
            Float(..) => KeyType::Float,
            String(..) => KeyType::String,
            Binary(..) => KeyType::Binary,
            Bool(..) => KeyType::Bool,
        }
    }
}

impl From<i64> for KeyData {
    fn from(v: i64) -> Self {
        KeyData::Int(v)
    }
}

impl From<f64> for KeyData {
    fn from(v: f64) -> Self {
        KeyData::Float(OrderedFloat(v))
    }
}

impl From<String> for KeyData {
    fn from(v: String) -> Self {
        KeyData::String(v)
    }
}

impl<'a> From<&'a str> for KeyData {
    fn from(v: &str) -> Self {
        KeyData::String(v.into())
    }
}

impl<'a> From<&'a [u8]> for KeyData {
    fn from(v: &[u8]) -> Self {
        KeyData::Binary(v.into())
    }
}

impl From<Vec<u8>> for KeyData {
    fn from(v: Vec<u8>) -> Self {
        KeyData::Binary(v)
    }
}

impl From<bool> for KeyData {
    fn from(v: bool) -> Self {
        KeyData::Bool(v)
    }
}

#[cfg(test)]
mod test {
    use super::{KeyType, KeyData};

    #[test]
    fn get_type() {
        assert_eq!(KeyData::from(123).get_type(), KeyType::Int);
        assert_eq!(KeyData::from(12.3).get_type(), KeyType::Float);
        assert_eq!(KeyData::from("abc").get_type(), KeyType::String);
        assert_eq!(KeyData::from(vec![1u8, 2, 3]).get_type(), KeyType::Binary);
        assert_eq!(KeyData::from(true).get_type(), KeyType::Bool);
    }

    #[test]
    fn as_type() {
        assert_eq!(KeyData::from("abc").as_type(KeyType::String).unwrap().get_type(), KeyType::String);
        assert_eq!(KeyData::from("abc").as_type(KeyType::Int), None);
        assert_eq!(KeyData::from(123).as_type(KeyType::Int).unwrap().get_type(), KeyType::Int);
        assert_eq!(KeyData::from(123).as_type(KeyType::Float), None);
        assert_eq!(KeyData::from(12.3).as_type(KeyType::Float).unwrap().get_type(), KeyType::Float);
        assert_eq!(KeyData::from(12.3).as_type(KeyType::Int), None);
        assert_eq!(KeyData::from(true).as_type(KeyType::Bool).unwrap().get_type(), KeyType::Bool);
        assert_eq!(KeyData::from(true).as_type(KeyType::String), None);
    }

    #[test]
    fn into_type() {
        assert_eq!(KeyData::from("abc").into_type(KeyType::String).unwrap().get_type(), KeyType::String);
        assert_eq!(KeyData::from("abc").into_type(KeyType::Int), None);
        assert_eq!(KeyData::from("123").into_type(KeyType::Int).unwrap().into_owned(), KeyData::from(123));
        assert_eq!(KeyData::from("12.3").into_type(KeyType::Float).unwrap().into_owned(), KeyData::from(12.3));
        assert_eq!(KeyData::from("12.3").into_type(KeyType::Int), None);
        assert_eq!(KeyData::from(123).into_type(KeyType::Int).unwrap().get_type(), KeyType::Int);
        assert_eq!(KeyData::from(123).into_type(KeyType::Float).unwrap().into_owned(), KeyData::from(123.0));
        assert_eq!(KeyData::from(123).into_type(KeyType::String).unwrap().into_owned(), KeyData::from("123"));
        assert_eq!(KeyData::from(12.3).into_type(KeyType::Float).unwrap().into_owned(), KeyData::from(12.3));
        assert_eq!(KeyData::from(12.3).into_type(KeyType::Int).unwrap().into_owned(), KeyData::from(12));
        assert_eq!(KeyData::from(12.5).into_type(KeyType::Int).unwrap().into_owned(), KeyData::from(13));
        assert_eq!(KeyData::from(12.3).into_type(KeyType::String).unwrap().into_owned(), KeyData::from("12.3"));
        assert_eq!(KeyData::from(true).into_type(KeyType::Bool).unwrap().get_type(), KeyType::Bool);
        assert_eq!(KeyData::from(true).into_type(KeyType::String).unwrap().into_owned(), KeyData::from("true"));
    }
}