use std::{
    collections::HashMap,
    iter::once,
    ops::Deref,
    result::Result as StdResult,
    f64::EPSILON,
};

use regex::Regex;
use serde::{
    de::{Deserializer, Error as DeError},
    ser::{SerializeMap, Serializer},
    Deserialize, Serialize,
};

use super::{Identifier, Value};

/// Modifier action
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Action {
    /// Set new value to field
    #[serde(rename = "$set")]
    Set(Value),
    /// Delete field
    #[serde(rename = "$delete")]
    Delete,
    /// Add some value to field
    ///
    /// This also works with string and bytes fields
    #[serde(rename = "$add")]
    Add(Value),
    /// Substract some value from field
    #[serde(rename = "$sub")]
    Sub(Value),
    /// Multiply field to value
    #[serde(rename = "$mul")]
    Mul(Value),
    /// Divide field to value
    #[serde(rename = "$div")]
    Div(Value),
    /// Toggle boolean field
    #[serde(rename = "$toggle")]
    Toggle,
    /// Replace string field using regular expression
    #[serde(rename = "$replace")]
    Replace(WrappedRegex, String),
    /// Splice array field
    #[serde(rename = "$splice")]
    #[serde(with = "splice")]
    Splice(i32, i32, Vec<Value>),
    /// Merge object field
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

/// Modification operator
///
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Modify(pub HashMap<Identifier, Vec<Action>>);

impl Serialize for Modify {
    fn serialize<S: Serializer>(&self, serializer: S) -> StdResult<S::Ok, S::Error> {
        let mut map = serializer.serialize_map(Some(self.0.len()))?;
        for (field, actions) in &self.0 {
            if !actions.is_empty() {
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
        Ok(Modify(
            map.into_iter()
                .map(|(field, actions)| {
                    (
                        field.into(),
                        match actions {
                            SingleOrMultiple::Multiple(v) => v,
                            SingleOrMultiple::Single(v) => vec![v],
                        },
                    )
                }).collect(),
        ))
    }
}

impl Modify {
    /// Create empty modifier
    pub fn new() -> Self {
        Self::default()
    }

    /// Create single modifier
    pub fn one<I: Into<Identifier>>(field: I, action: Action) -> Self {
        Modify(once((field.into(), vec![action])).collect())
    }

    /// Append modification to modifier
    pub fn add<I: Into<Identifier>>(&mut self, field: I, action: Action) {
        let field = field.into();
        let entry = self.0.entry(field).or_default();
        entry.push(action);
    }

    /// Add modification to modifier
    pub fn with<I: Into<Identifier>>(mut self, field: I, action: Action) -> Self {
        self.add(field, action);
        self
    }

    /// Apply modifier to generic data
    pub fn apply(&self, val: Value) -> Value {
        modify_value(&self.0, "", val)
    }
}

fn modify_value(mods: &HashMap<Identifier, Vec<Action>>, pfx: &str, mut val: Value) -> Value {
    use Value::*;

    if let Some(acts) = mods.get(pfx) {
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
        Integer(_) => modify_primitive(mods, pfx, val),
        Float(_) => modify_primitive(mods, pfx, val),
        Text(_) => modify_primitive(mods, pfx, val),
        Bool(_) => modify_primitive(mods, pfx, val),
        Bytes(_) => modify_primitive(mods, pfx, val),
        Array(mut vec) => {
            if let Some(acts) = mods.get(pfx) {
                for act in acts {
                    use Action::*;
                    match act {
                        Add(Array(elms)) => {
                            for elm in elms {
                                if !vec.iter().any(|e| e == elm) {
                                    vec.push(elm.clone());
                                }
                            }
                        }
                        Sub(Array(elms)) => {
                            for elm in elms {
                                if let Some(idx) = vec.iter().position(|e| e == elm) {
                                    vec.remove(idx);
                                }
                            }
                        }
                        Splice(off, del, ins) => {
                            let beg = usize::min(
                                if *off >= 0 {
                                    *off as usize
                                } else {
                                    vec.len() - (-1 - *off) as usize
                                },
                                vec.len(),
                            );
                            let end = usize::min(
                                if *del >= 0 {
                                    *del as usize
                                } else {
                                    vec.len() - (-1 - *del) as usize
                                },
                                vec.len(),
                            );
                            vec.splice(beg..end, ins.iter().cloned());
                        }
                        _ => (),
                    }
                }
            }
            Array(
                vec.into_iter()
                    .map(|elm| modify_value(mods, pfx, elm))
                    .collect(),
            )
        }
        Map(mut map) => {
            if let Some(acts) = mods.get(pfx) {
                for act in acts {
                    use Action::*;
                    #[allow(clippy::single_match)]
                    match act {
                        Merge(Map(obj)) =>
                            map.extend(obj.iter().map(|(k, v)| (k.clone(), v.clone()))),
                        _ => (),
                    }
                }
            }

            Map(
                map.into_iter()
                    .map(|(key, val)| {
                        let field = nested_field(pfx, &key);
                        (key, modify_value(mods, &field, val))
                    }).collect(),
            )
        }
        other => other,
    }
}

fn modify_primitive(mods: &HashMap<Identifier, Vec<Action>>, pfx: &str, mut val: Value) -> Value {
    use Value::*;

    if let Some(acts) = mods.get(pfx) {
        for act in acts {
            use Action::*;
            val = match (&val, act) {
                // add
                (Integer(pre), Add(Integer(arg))) => Integer(pre + arg),
                (Integer(pre), Add(Float(arg))) => Float(*pre as f64 + arg),
                (Float(pre), Add(Integer(arg))) => Float(pre + *arg as f64),
                (Float(pre), Add(Float(arg))) => Float(pre + arg),

                // substract
                (Integer(pre), Sub(Integer(arg))) => Integer(pre - arg),
                (Integer(pre), Sub(Float(arg))) => Float(*pre as f64 - arg),
                (Float(pre), Sub(Integer(arg))) => Float(pre - *arg as f64),
                (Float(pre), Sub(Float(arg))) => Float(pre - arg),

                // multiply
                (Integer(pre), Mul(Integer(arg))) => Integer(pre * arg),
                (Integer(pre), Mul(Float(arg))) => Float(*pre as f64 * arg),
                (Float(pre), Mul(Integer(arg))) => Float(pre * *arg as f64),
                (Float(pre), Mul(Float(arg))) => Float(pre * arg),

                // divide
                (Integer(pre), Div(Integer(arg))) => Integer(pre / arg),
                (Integer(pre), Div(Float(arg))) => Float(*pre as f64 / arg),
                (Float(pre), Div(Integer(arg))) => Float(pre / *arg as f64),
                (Float(pre), Div(Float(arg))) => Float(pre / arg),

                // toggle
                (Bool(pre), Toggle) => Bool(!pre),

                // concat
                (Text(pre), Add(Text(arg))) => Text(pre.clone() + arg),

                (Bytes(pre), Add(Bytes(arg))) => {
                    let mut vec = pre.clone();
                    vec.append(&mut arg.clone());
                    Bytes(vec)
                }

                // replace
                (Text(pre), Replace(regex, subst)) => {
                    Text(regex.replace_all(pre, subst.as_str()).into())
                }

                _ => continue,
            };
        }
    }

    match val {
        Float(v) if (v.trunc() - v).abs() < EPSILON => Integer(v as i128),
        _ => val,
    }
}

fn nested_field(pfx: &str, key: &Value) -> String {
    use self::Value::*;
    match key {
        Text(s) => pfx.to_owned() + if pfx.is_empty() { "" } else { "." } + s,
        Integer(i) => pfx.to_owned() + "[" + &i.to_string() + "]",
        _ => pfx.into(),
    }
}

mod splice {
    use super::Value;
    use serde::{de, ser::SerializeSeq, Deserialize, Deserializer, Serializer};

    #[allow(clippy::ptr_arg, clippy::trivially_copy_pass_by_ref)]
    pub fn serialize<S: Serializer>(
        off: &i32,
        del: &i32,
        ins: &Vec<Value>,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        let mut map = serializer.serialize_seq(Some(2 + ins.len()))?;
        map.serialize_element(off)?;
        map.serialize_element(del)?;
        for elm in ins {
            map.serialize_element(elm)?;
        }
        map.end()
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<(i32, i32, Vec<Value>), D::Error> {
        let seq: Vec<Value> = Vec::deserialize(deserializer)?;
        let mut it = seq.into_iter();
        use Value::*;
        match (it.next(), it.next()) {
            (Some(Integer(off)), Some(Integer(del))) => Ok((off as i32, del as i32, it.collect())),
            _ => Err(de::Error::custom("Invalid $slice op")),
        }
    }
}

#[cfg(test)]
mod test {
    use super::{Action, Modify};
    use serde_json::{from_str, value::from_value, to_string, Value, json};

    #[test]
    fn parse_field_set_single() {
        test_parse!(
            Modify,
            json!({ "field": { "$set": 0 } }),
            Modify::one("field", Action::Set(json_val!(0)))
        );

        test_parse!(
            Modify,
            json!({ "field": { "$set": "abc" } }),
            Modify::one("field", Action::Set(json_val!("abc")))
        );

        test_parse!(
            Modify,
            json!({ "field": { "$set": { "a": 99 } } }),
            Modify::one("field", Action::Set(json_val!({ "a": 99 })))
        );
    }

    #[test]
    fn build_field_set_one() {
        test_build!(
            Modify::one("field", Action::Set(json_val!(0))),
            json!({ "field": { "$set": 0 } })
        );

        test_build!(
            Modify::one("field", Action::Set(json_val!("abc"))),
            json!({ "field": { "$set": "abc" } })
        );

        test_build!(
            Modify::one("field", Action::Set(json_val!({ "a": 99 }))),
            json!({ "field": { "$set": { "a": 99 } } })
        );
    }

    #[test]
    fn parse_field_set_multi() {
        test_parse!(
            Modify,
            json!({ "field": { "$set": 0 }, "string": { "$set": "abc" } }),
            Modify::one("field", Action::Set(json_val!(0)))
                .with("string", Action::Set(json_val!("abc")))
        );
    }

    #[test]
    fn build_field_set_multi() {
        test_build!(
            Modify::one("field", Action::Set(json_val!(0)))
                .with("string", Action::Set(json_val!("abc"))),
            json!({ "field": { "$set": 0 }, "string": { "$set": "abc" } })
        );
    }

    #[test]
    fn field_set() {
        let m: Modify = json_val!({ "field": { "$set": 123 } });

        assert_eq!(
            m.apply(json_val!({ "field": "abc" })),
            json_val!({ "field": 123 })
        );
    }

    #[test]
    fn field_delete() {
        let m: Modify = json_val!({ "field": "$delete" });

        assert_eq!(
            m.apply(json_val!({ "field": "abc" })),
            json_val!({ "field": null })
        );
    }

    #[test]
    fn sub_field_set() {
        let m: Modify = json_val!({ "obj.str": { "$set": "def" } });

        assert_eq!(
            m.apply(json_val!({ "obj": { "str": "abc" } })),
            json_val!({ "obj": { "str": "def" } })
        );
    }

    #[test]
    fn sub_field_delete() {
        let m: Modify = json_val!({ "obj.key": "$delete" });

        assert_eq!(
            m.apply(json_val!({ "obj": { "key": "abc" } })),
            json_val!({ "obj": { "key": null } })
        );
    }

    #[test]
    fn numeric_add() {
        let m: Modify = json_val!({ "counter": { "$add": 1 } });

        assert_eq!(
            m.apply(json_val!({ "counter": 0 })),
            json_val!({ "counter": 1 })
        );
    }

    #[test]
    fn numeric_sub() {
        let m: Modify = json_val!({ "counter": { "$add": -1 } });

        assert_eq!(
            m.apply(json_val!({ "counter": 10 })),
            json_val!({ "counter": 9 })
        );
    }

    #[test]
    fn string_replace() {
        let m: Modify = json_val!({ "str": { "$replace": ["bra", "3"] } });

        assert_eq!(
            m.apply(json_val!({ "str": "abracadabra" })),
            json_val!({ "str": "a3cada3" })
        );
    }

    #[test]
    fn array_add() {
        let m: Modify = json_val!({ "list": { "$add": [2, 3, 4] } });

        assert_eq!(
            m.apply(json_val!({ "list": [1, 3, 5] })),
            json_val!({ "list": [1, 3, 5, 2, 4] })
        );
    }

    #[test]
    fn array_sub() {
        let m: Modify = json_val!({ "list": { "$sub": [2, 3, 5] } });

        assert_eq!(
            m.apply(json_val!({ "list": [1, 2, 4, 5] })),
            json_val!({ "list": [1, 4] })
        );
    }

    #[test]
    fn array_prepend() {
        let m: Modify = json_val!({ "list": { "$splice": [0, 0, 1, 2] } });

        assert_eq!(
            m.apply(json_val!({ "list": [3, 4, 5] })),
            json_val!({ "list": [1, 2, 3, 4, 5] })
        );
    }

    #[test]
    fn array_append() {
        let m: Modify = json_val!({ "list": { "$splice": [-1, -1, 1, 2] } });

        assert_eq!(
            m.apply(json_val!({ "list": [3, 4, 5] })),
            json_val!({ "list": [3, 4, 5, 1, 2] })
        );
    }

    #[test]
    fn array_splice() {
        let m: Modify = json_val!({ "list": { "$splice": [1, 3, 0] } });

        assert_eq!(
            m.apply(json_val!({ "list": [1, 2, 3, 4, 5] })),
            json_val!({ "list": [1, 0, 4, 5] })
        );

        let m: Modify = json_val!({ "list": { "$splice": [-4, -1, 0, -1] } });

        assert_eq!(
            m.apply(json_val!({ "list": [1, 2, 3, 4, 5] })),
            json_val!({ "list": [1, 2, 0, -1] })
        );
    }

    #[test]
    fn object_merge() {
        let m: Modify = json_val!({ "obj": { "$merge": { "a": 2, "b": "a" } } });

        assert_eq!(
            m.apply(json_val!({ "obj": { "a": 1, "c": true } })),
            json_val!({ "obj": { "a": 2, "b": "a", "c": true } })
        );
    }
}
