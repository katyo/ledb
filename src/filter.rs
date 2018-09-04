/*
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Atom {
    Bool(bool),
    Int(i64),
    String(String),
}
 */
use index::IndexData as Atom;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Comp {
    #[serde(rename = "$eq")]
    Eq(Atom),
    #[serde(rename = "$lt")]
    Lt(Atom),
    #[serde(rename = "$gt")]
    Gt(Atom),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Cond {
    #[serde(rename = "$not")]
    Not(Box<Filter>),
    #[serde(rename = "$and")]
    And(Vec<Filter>),
    #[serde(rename = "$or")]
    Or(Vec<Filter>),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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
    use super::{Filter, Comp, Cond, Atom};
    use serde_json::{from_str, to_string };

    #[test]
    fn parse_comp_eq() {
        assert_eq!(from_str::<Filter>(r#"{"field":{"$eq":0}}"#).unwrap(),
                   Filter::Comp("field".into(), Comp::Eq(Atom::Int(0))));
        assert_eq!(from_str::<Filter>(r#"{"name":{"$eq":"vlada"}}"#).unwrap(),
                   Filter::Comp("name".into(), Comp::Eq(Atom::String("vlada".into()))));
    }

    #[test]
    fn build_comp_eq() {
        assert_eq!(to_string(&Filter::Comp("field".into(), Comp::Eq(Atom::Int(0)))).unwrap(),
                   r#"{"field":{"$eq":0}}"#);
        assert_eq!(to_string(&Filter::Comp("name".into(), Comp::Eq(Atom::String("vlada".into())))).unwrap(),
                   r#"{"name":{"$eq":"vlada"}}"#);
    }

    #[test]
    fn parse_cond_not() {
        assert_eq!(from_str::<Filter>(r#"{"$not":{"a":{"$gt":9}}}"#).unwrap(),
                   Filter::Cond(Cond::Not(
                       Box::new(Filter::Comp("a".into(), Comp::Gt(Atom::Int(9)))),
                   )));
    }

    #[test]
    fn build_cond_not() {
        assert_eq!(to_string(&Filter::Cond(Cond::Not(
            Box::new(Filter::Comp("a".into(), Comp::Gt(Atom::Int(9))))
        ))).unwrap(), r#"{"$not":{"a":{"$gt":9}}}"#);
    }

    #[test]
    fn parse_cond_and() {
        assert_eq!(from_str::<Filter>(r#"{"$and":[{"a":{"$eq":11}},{"b":{"$lt":-1}}]}"#).unwrap(),
                   Filter::Cond(Cond::And(vec![
                       Filter::Comp("a".into(), Comp::Eq(Atom::Int(11))),
                       Filter::Comp("b".into(), Comp::Lt(Atom::Int(-1))),
                   ])));
    }

    #[test]
    fn build_cond_and() {
        assert_eq!(to_string(&Filter::Cond(Cond::And(vec![
            Filter::Comp("a".into(), Comp::Eq(Atom::Int(11))),
            Filter::Comp("b".into(), Comp::Lt(Atom::Int(-1))),
        ]))).unwrap(), r#"{"$and":[{"a":{"$eq":11}},{"b":{"$lt":-1}}]}"#);
    }

    #[test]
    fn parse_cond_or() {
        assert_eq!(from_str::<Filter>(r#"{"$or":[{"a":{"$eq":11}},{"b":{"$lt":-1}}]}"#).unwrap(),
                   Filter::Cond(Cond::Or(vec![
                       Filter::Comp("a".into(), Comp::Eq(Atom::Int(11))),
                       Filter::Comp("b".into(), Comp::Lt(Atom::Int(-1))),
                   ])));
    }

    #[test]
    fn build_cond_or() {
        assert_eq!(to_string(&Filter::Cond(Cond::Or(vec![
            Filter::Comp("a".into(), Comp::Eq(Atom::Int(11))),
            Filter::Comp("b".into(), Comp::Lt(Atom::Int(-1))),
        ]))).unwrap(), r#"{"$or":[{"a":{"$eq":11}},{"b":{"$lt":-1}}]}"#);
    }
}
