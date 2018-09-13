use std::ops::Deref;
use std::collections::HashMap;
use std::result::Result as StdResult;
use std::iter::once;

use serde::{Serialize, ser::{Serializer, SerializeMap}, Deserialize, de::{Deserializer, Error as DeError}};
use serde_cbor::{ObjectKey};
use regex::{Regex};

use super::{Value};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Action {
    #[serde(rename = "$set")]
    Set(Value),
    #[serde(rename = "$delete")]
    Delete,
    #[serde(rename = "$add")]
    Add(Value),
    #[serde(rename = "$mul")]
    Mul(Value),
    #[serde(rename = "$toggle")]
    Toggle,
    #[serde(rename = "$replace")]
    Replace(WrappedRegex, String),
    #[serde(rename = "$prepend")]
    Prepend(Vec<Value>),
    #[serde(rename = "$append")]
    Append(Vec<Value>),
    #[serde(rename = "$splice")]
    #[serde(with = "splice")]
    Splice(i32, u32, Vec<Value>),
    #[serde(rename = "$merge")]
    Merge(Value),
}

#[derive(Debug, Clone)]
pub struct WrappedRegex(pub Regex);

impl PartialEq for WrappedRegex {
    fn eq(&self, other: &Self) -> bool {
        format!("{}", self.0) == format!("{}", other.0)
    }
}

impl Deref for WrappedRegex {
    type Target = Regex;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Serialize for WrappedRegex {
    fn serialize<S: Serializer>(&self, serializer: S) -> StdResult<S::Ok, S::Error> {
        serializer.serialize_str(&format!("{}", self.0))
    }
}

impl<'de> Deserialize<'de> for WrappedRegex {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> StdResult<Self, D::Error> {
        // <&str>::deserialize did not works...
        String::deserialize(deserializer)?
            .parse()
            .map(WrappedRegex)
            .map_err(|e| D::Error::custom(format!("{}", e)))
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Modify(pub HashMap<String, Vec<Action>>);

impl Serialize for Modify {
    fn serialize<S: Serializer>(&self, serializer: S) -> StdResult<S::Ok, S::Error> {
        let mut map = serializer.serialize_map(Some(self.0.len()))?;
        for (field, actions) in &self.0 {
            if actions.len() > 0 {
                if actions.len() == 1 {
                    map.serialize_entry(field, &actions[0])?;
                } else {
                    map.serialize_entry(field, actions)?;
                }
            }
        }
        map.end()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
enum SingleOrMultiple<T> {
    Multiple(Vec<T>),
    Single(T),
}

impl<'de> Deserialize<'de> for Modify {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> StdResult<Self, D::Error> {
        let map: HashMap<String, SingleOrMultiple<Action>> = HashMap::deserialize(deserializer)?;
        Ok(Modify(map.into_iter().map(|(field, actions)| (field, match actions {
            SingleOrMultiple::Multiple(v) => v,
            SingleOrMultiple::Single(v) => vec![v],
        })).collect()))
    }
}

impl Modify {
    pub fn new() -> Self {
        Self::default()
    }
    
    pub fn single<S: AsRef<str>>(field: S, action: Action) -> Self {
        Modify(once((field.as_ref().into(), vec![action])).collect())
    }

    pub fn multiple<S: AsRef<str>, M: AsRef<[(S, Action)]>>(mods: M) -> Self {
        let mut ms = Self::default();

        for (field, action) in mods.as_ref() {
            ms.add(field, action.clone());
        }
        
        ms
    }

    pub fn add<S: AsRef<str>>(&mut self, field: S, action: Action) {
        let field = field.as_ref().into();
        let entry = self.0.entry(field).or_default();
        entry.push(action);
    }

    pub fn with<S: AsRef<str>>(mut self, field: S, action: Action) -> Self {
        self.add(field, action);
        self
    }

    pub fn apply(&self, val: Value) -> Value {
        modify_value(&self.0, "", val)
    }
}

fn modify_value(mods: &HashMap<String, Vec<Action>>, pfx: &str, mut val: Value) -> Value {
    use Value::*;

    if let Some(acts) = mods.get(pfx.into()) {
        for act in acts {
            use Action::*;
            val = match act {
                Set(new) => new.clone(),
                Delete => Null,
                _ => continue,
            };
        }
    }
    
    match val {
        U64(_) => modify_primitive(mods, pfx, val),
        I64(_) => modify_primitive(mods, pfx, val),
        F64(_) => modify_primitive(mods, pfx, val),
        String(_) => modify_primitive(mods, pfx, val),
        Bool(_) => modify_primitive(mods, pfx, val),
        Bytes(_) => modify_primitive(mods, pfx, val),
        Array(mut vec) => {
            if let Some(acts) = mods.get(pfx.into()) {
                for act in acts {
                    use Action::*;
                    match act {
                        Prepend(elms) => {
                            vec.splice(0..0, elms.iter().cloned());
                        },
                        Append(elms) => {
                            let end = vec.len();
                            vec.splice(end..end, elms.iter().cloned());
                        },
                        Splice(off, del, ins) => {
                            let beg = usize::min(if *off >= 0 { *off as usize } else { vec.len() - (-1 - *off) as usize }, vec.len());
                            let end = usize::min(beg + *del as usize, vec.len());
                            vec.splice(beg .. end, ins.iter().cloned());
                        },
                        _ => (),
                    }
                }
            }
            Array(vec.into_iter().map(|elm| modify_value(mods, pfx, elm)).collect())
        },
        Object(mut map) => {
            if let Some(acts) = mods.get(pfx.into()) {
                for act in acts {
                    use Action::*;
                    match act {
                        Merge(Object(obj)) => {
                            map.extend(obj.iter().map(|(k, v)| (k.clone(), v.clone())));
                        },
                        _ => (),
                    }
                }
            }
            
            Object(map.into_iter().map(|(key, val)| {
                let field = nested_field(pfx, &key);
                (key, modify_value(mods, &field, val))
            }).collect())
        },
        Null => Null,
    }
}

fn modify_primitive(mods: &HashMap<String, Vec<Action>>, pfx: &str, mut val: Value) -> Value {
    use Value::*;
    
    if let Some(acts) = mods.get(pfx.into()) {
        for act in acts {
            use Action::*;
            val = match (&val, act) {
                // add
                (U64(pre), Add(U64(arg))) => U64(pre + arg),
                (U64(pre), Add(I64(arg))) => I64(*pre as i64 + arg),
                (U64(pre), Add(F64(arg))) => F64(*pre as f64 + arg),
                
                (I64(pre), Add(U64(arg))) => I64(pre + *arg as i64),
                (I64(pre), Add(I64(arg))) => I64(pre + arg),
                (I64(pre), Add(F64(arg))) => F64(*pre as f64 + arg),
                
                (F64(pre), Add(U64(arg))) => F64(pre + *arg as f64),
                (F64(pre), Add(I64(arg))) => F64(pre + *arg as f64),
                (F64(pre), Add(F64(arg))) => F64(pre + arg),
                
                // multiply
                (U64(pre), Mul(U64(arg))) => U64(pre * arg),
                (U64(pre), Mul(I64(arg))) => I64(*pre as i64 * arg),
                (U64(pre), Mul(F64(arg))) => F64(*pre as f64 * arg),
                
                (I64(pre), Mul(U64(arg))) => I64(pre * *arg as i64),
                (I64(pre), Mul(I64(arg))) => I64(pre * arg),
                (I64(pre), Mul(F64(arg))) => F64(*pre as f64 * arg),
                
                (F64(pre), Mul(U64(arg))) => F64(pre * *arg as f64),
                (F64(pre), Mul(I64(arg))) => F64(pre * *arg as f64),
                (F64(pre), Mul(F64(arg))) => F64(pre * arg),

                // toggle
                (Bool(pre), Toggle) => Bool(!pre),

                // concat
                (String(pre), Add(String(arg))) => String(pre.clone() + arg),
                
                (Bytes(pre), Add(Bytes(arg))) => {
                    let mut vec = pre.clone();
                    vec.append(&mut arg.clone());
                    Bytes(vec)
                },
                
                // replace
                (String(pre), Replace(regex, subst)) => String(regex.replace_all(pre, subst.as_str()).into()),
                
                _ => continue,
            };
        }
    }
    
    match val {
        I64(v) if v > 0 => U64(v as u64),
        F64(v) if (v as u64) as f64 == v => U64(v as u64),
        F64(v) if (v as i64) as f64 == v => I64(v as i64),
        _ => val,
    }
}

fn nested_field(pfx: &str, key: &ObjectKey) -> String {
    use self::ObjectKey::*;
    match key {
        String(s) => pfx.to_owned() + if pfx.len() > 0 { "." } else { "" } + s,
        Integer(i) => pfx.to_owned() + "[" + &i.to_string() + "]",
        _ => pfx.into(),
    }
}

mod splice {
    use super::{Value};
    use serde::{Serializer, Deserializer, Deserialize, de::{self}, ser::{SerializeSeq}};
    
    pub fn serialize<S: Serializer>(off: &i32, del: &u32, ins: &Vec<Value>, serializer: S) -> Result<S::Ok, S::Error> {
        let mut map = serializer.serialize_seq(Some(2 + ins.len()))?;
        map.serialize_element(off)?;
        map.serialize_element(del)?;
        for elm in ins {
            map.serialize_element(elm)?;
        }
        map.end()
    }
    
    pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<(i32, u32, Vec<Value>), D::Error> {
        let seq: Vec<Value> = Vec::deserialize(deserializer)?;
        let mut it = seq.into_iter();
        use Value::*;
        match (it.next(), it.next()) {
            (Some(U64(off)), Some(U64(del))) => Ok((off as i32, del as u32, it.collect())),
            (Some(I64(off)), Some(U64(del))) => Ok((off as i32, del as u32, it.collect())),
            _ => Err(de::Error::custom("Invalid $slice op"))
        }
    }
}

#[cfg(test)]
mod test {
    use super::{Modify, Action};
    use serde_json::{from_str, to_string, from_value, Value};

    #[test]
    fn parse_field_set_single() {
        test_parse!(Modify, json!({ "field": { "$set": 0 } }),
                    Modify::single("field", Action::Set(json_val!(0))));

        test_parse!(Modify, json!({ "field": { "$set": "abc" } }),
                    Modify::single("field", Action::Set(json_val!("abc"))));

        test_parse!(Modify, json!({ "field": { "$set": { "a": 99 } } }),
                    Modify::single("field", Action::Set(json_val!({ "a": 99 }))));
    }

    #[test]
    fn build_field_set_single() {
        test_build!(Modify::single("field", Action::Set(json_val!(0))),
                    json!({ "field": { "$set": 0 } }));

        test_build!(Modify::single("field", Action::Set(json_val!("abc"))),
                    json!({ "field": { "$set": "abc" } }));

        test_build!(Modify::single("field", Action::Set(json_val!({ "a": 99 }))),
                    json!({ "field": { "$set": { "a": 99 } } }));
    }

    #[test]
    fn parse_field_set_multi() {
        test_parse!(Modify, json!({ "field": { "$set": 0 }, "string": { "$set": "abc" } }),
                    Modify::single("field", Action::Set(json_val!(0)))
                    .with("string", Action::Set(json_val!("abc"))));
    }

    #[test]
    fn build_field_set_multi() {
        test_build!(Modify::single("field", Action::Set(json_val!(0)))
                    .with("string", Action::Set(json_val!("abc"))),
                    json!({ "field": { "$set": 0 }, "string": { "$set": "abc" } }));
    }

    #[test]
    fn field_set() {
        let m: Modify = json_val!({ "field": { "$set": 123 } });

        assert_eq!(m.apply(json_val!({ "field": "abc" })),
                   json_val!({ "field": 123 }));
    }

    #[test]
    fn field_delete() {
        let m: Modify = json_val!({ "field": "$delete" });

        assert_eq!(m.apply(json_val!({ "field": "abc" })),
                   json_val!({ "field": null }));
    }

    #[test]
    fn sub_field_set() {
        let m: Modify = json_val!({ "obj.str": { "$set": "def" } });

        assert_eq!(m.apply(json_val!({ "obj": { "str": "abc" } })),
                   json_val!({ "obj": { "str": "def" } }));
    }

    #[test]
    fn sub_field_delete() {
        let m: Modify = json_val!({ "obj.key": "$delete" });

        assert_eq!(m.apply(json_val!({ "obj": { "key": "abc" } })),
                   json_val!({ "obj": { "key": null } }));
    }

    #[test]
    fn numeric_add() {
        let m: Modify = json_val!({ "counter": { "$add": 1 } });

        assert_eq!(m.apply(json_val!({ "counter": 0 })),
                   json_val!({ "counter": 1 }));
    }

    #[test]
    fn numeric_sub() {
        let m: Modify = json_val!({ "counter": { "$add": -1 } });

        assert_eq!(m.apply(json_val!({ "counter": 10 })),
                   json_val!({ "counter": 9 }));
    }

    #[test]
    fn string_replace() {
        let m: Modify = json_val!({ "str": { "$replace": ["bra", "3"] } });

        assert_eq!(m.apply(json_val!({ "str": "abracadabra" })),
                   json_val!({ "str": "a3cada3" }));
    }

    #[test]
    fn array_prepend() {
        let m: Modify = json_val!({ "list": { "$prepend": [1, 2] } });

        assert_eq!(m.apply(json_val!({ "list": [3, 4, 5] })),
                   json_val!({ "list": [1, 2, 3, 4, 5] }));
    }

    #[test]
    fn array_append() {
        let m: Modify = json_val!({ "list": { "$append": [1, 2] } });

        assert_eq!(m.apply(json_val!({ "list": [3, 4, 5] })),
                   json_val!({ "list": [3, 4, 5, 1, 2] }));
    }

    #[test]
    fn array_splice() {
        let m: Modify = json_val!({ "list": { "$splice": [1, 2, 0] } });

        assert_eq!(m.apply(json_val!({ "list": [1, 2, 3, 4, 5] })),
                   json_val!({ "list": [1, 0, 4, 5] }));

        let m: Modify = json_val!({ "list": { "$splice": [-4, 3, 0, -1] } });

        assert_eq!(m.apply(json_val!({ "list": [1, 2, 3, 4, 5] })),
                   json_val!({ "list": [1, 2, 0, -1] }));
    }

    #[test]
    fn object_merge() {
        let m: Modify = json_val!({ "obj": { "$merge": { "a": 2, "b": "a" } } });

        assert_eq!(m.apply(json_val!({ "obj": { "a": 1, "c": true } })),
                   json_val!({ "obj": { "a": 2, "b": "a", "c": true } }));
    }
}
