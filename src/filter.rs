use std::str::from_utf8;
use std::mem::transmute;
use byteorder::{ByteOrder, NativeEndian};

use types::{ResultWrap};
use document::{Value};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum KeyData {
    Int(i64),
    Float(f64),
    String(String),
    Binary(Vec<u8>),
    Bool(bool),
}

impl KeyData {
    pub fn from_raw(typ: &KeyType, raw: &[u8]) -> Result<Self, String> {
        use self::KeyData::*;
        Ok(match typ {
            KeyType::Int => {
                if raw.len() != 8 { return Err("Int key must be 8 bytes length".into()) }
                Int(NativeEndian::read_i64(raw))
            },
            KeyType::Float => {
                if raw.len() != 8 { return Err("Float key must be 8 bytes length".into()) }
                Float(NativeEndian::read_f64(raw))
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
            F64(val) => KeyData::Float(*val),
            Bytes(val) => KeyData::Binary(val.clone()),
            String(val) => KeyData::String(val.clone()),
            Bool(val) => KeyData::Bool(*val),
            _ => return None,
        })
    }

    pub fn cast_type<'a>(&'a self, typ: &KeyType) -> Option<&'a KeyData> {
        use self::KeyData::*;
        match (typ, self) {
            (KeyType::Int, Int(..)) => Some(self),
            (KeyType::Float, Float(..)) => Some(self),
            (KeyType::Binary, Binary(..)) => Some(self),
            (KeyType::String, String(..)) => Some(self),
            (KeyType::Bool, Bool(..)) => Some(self),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Comp {
    #[serde(rename = "$eq")]
    Eq(KeyData),
    #[serde(rename = "$lt")]
    Lt(KeyData),
    #[serde(rename = "$gt")]
    Gt(KeyData),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Cond {
    #[serde(rename = "$not")]
    Not(Box<Filter>),
    #[serde(rename = "$and")]
    And(Vec<Filter>),
    #[serde(rename = "$or")]
    Or(Vec<Filter>),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Filter {
    Cond(Cond),
    #[serde(with = "comp")]
    Comp(String, Comp),
}

mod comp {
    use super::{Comp};
    use std::collections::HashMap;
    use serde::{Serializer, Deserializer, Deserialize, de::{self}, ser::{SerializeMap}};
    
    pub fn serialize<S: Serializer>(field: &String, op: &Comp, serializer: S) -> Result<S::Ok, S::Error> {
        let mut map = serializer.serialize_map(Some(1))?;
        map.serialize_entry(&field, &op)?;
        map.end()
    }
    
    pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<(String, Comp), D::Error> {
        let map: HashMap<String, Comp> = HashMap::deserialize(deserializer)?;
        let mut it = map.into_iter();
        match (it.next(), it.next()) {
            (Some((field, op)), None) => Ok((field, op)),
            _ => Err(de::Error::custom("Not a comp op"))
        }
    }
}

#[cfg(test)]
mod test {
    use super::{Filter, Comp, Cond, KeyData};
    use serde_json::{from_str, to_string };

    #[test]
    fn parse_comp_eq() {
        assert_eq!(from_str::<Filter>(r#"{"field":{"$eq":0}}"#).unwrap(),
                   Filter::Comp("field".into(), Comp::Eq(KeyData::Int(0))));
        assert_eq!(from_str::<Filter>(r#"{"name":{"$eq":"vlada"}}"#).unwrap(),
                   Filter::Comp("name".into(), Comp::Eq(KeyData::String("vlada".into()))));
    }

    #[test]
    fn build_comp_eq() {
        assert_eq!(to_string(&Filter::Comp("field".into(), Comp::Eq(KeyData::Int(0)))).unwrap(),
                   r#"{"field":{"$eq":0}}"#);
        assert_eq!(to_string(&Filter::Comp("name".into(), Comp::Eq(KeyData::String("vlada".into())))).unwrap(),
                   r#"{"name":{"$eq":"vlada"}}"#);
    }

    #[test]
    fn parse_cond_not() {
        assert_eq!(from_str::<Filter>(r#"{"$not":{"a":{"$gt":9}}}"#).unwrap(),
                   Filter::Cond(Cond::Not(
                       Box::new(Filter::Comp("a".into(), Comp::Gt(KeyData::Int(9)))),
                   )));
    }

    #[test]
    fn build_cond_not() {
        assert_eq!(to_string(&Filter::Cond(Cond::Not(
            Box::new(Filter::Comp("a".into(), Comp::Gt(KeyData::Int(9))))
        ))).unwrap(), r#"{"$not":{"a":{"$gt":9}}}"#);
    }

    #[test]
    fn parse_cond_and() {
        assert_eq!(from_str::<Filter>(r#"{"$and":[{"a":{"$eq":11}},{"b":{"$lt":-1}}]}"#).unwrap(),
                   Filter::Cond(Cond::And(vec![
                       Filter::Comp("a".into(), Comp::Eq(KeyData::Int(11))),
                       Filter::Comp("b".into(), Comp::Lt(KeyData::Int(-1))),
                   ])));
    }

    #[test]
    fn build_cond_and() {
        assert_eq!(to_string(&Filter::Cond(Cond::And(vec![
            Filter::Comp("a".into(), Comp::Eq(KeyData::Int(11))),
            Filter::Comp("b".into(), Comp::Lt(KeyData::Int(-1))),
        ]))).unwrap(), r#"{"$and":[{"a":{"$eq":11}},{"b":{"$lt":-1}}]}"#);
    }

    #[test]
    fn parse_cond_or() {
        assert_eq!(from_str::<Filter>(r#"{"$or":[{"a":{"$eq":11}},{"b":{"$lt":-1}}]}"#).unwrap(),
                   Filter::Cond(Cond::Or(vec![
                       Filter::Comp("a".into(), Comp::Eq(KeyData::Int(11))),
                       Filter::Comp("b".into(), Comp::Lt(KeyData::Int(-1))),
                   ])));
    }

    #[test]
    fn build_cond_or() {
        assert_eq!(to_string(&Filter::Cond(Cond::Or(vec![
            Filter::Comp("a".into(), Comp::Eq(KeyData::Int(11))),
            Filter::Comp("b".into(), Comp::Lt(KeyData::Int(-1))),
        ]))).unwrap(), r#"{"$or":[{"a":{"$eq":11}},{"b":{"$lt":-1}}]}"#);
    }
}
