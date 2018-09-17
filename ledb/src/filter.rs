use std::iter::once;
use lmdb::{ReadTransaction};

use super::{Identifier, Result, KeyData, Selection, Collection};

/// Comparison operator of filter
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Comp {
    /// Equal
    #[serde(rename = "$eq")]
    Eq(KeyData),
    /// In set (equal one of)
    #[serde(rename = "$in")]
    In(Vec<KeyData>),
    /// Less than
    #[serde(rename = "$lt")]
    Lt(KeyData),
    /// Less than or equal
    #[serde(rename = "$le")]
    Le(KeyData),
    /// Greater than
    #[serde(rename = "$gt")]
    Gt(KeyData),
    /// Greater than or equal
    #[serde(rename = "$ge")]
    Ge(KeyData),
    /// Between (in range)
    #[serde(rename = "$bw")]
    Bw(KeyData, bool, KeyData, bool),
    /// Field exists (not null)
    #[serde(rename = "$has")]
    Has,
}

/// Condition operator of filter
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Cond {
    /// Not (sub-condition is false)
    #[serde(rename = "$not")]
    Not(Box<Filter>),
    /// And (all of sub-conditions is true)
    #[serde(rename = "$and")]
    And(Vec<Filter>),
    /// Or (any of sub-conditions is true)
    #[serde(rename = "$or")]
    Or(Vec<Filter>),
}

/// Filter operator
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Filter {
    /// Condition operator
    Cond(Cond),
    /// Comparison operator
    #[serde(with = "comp")]
    Comp(Identifier, Comp),
}

impl Filter {
    pub fn cond(cond: Cond) -> Self {
        Filter::Cond(cond)
    }

    pub fn comp<F: Into<Identifier>>(field: F, comp: Comp) -> Self {
        Filter::Comp(field.into(), comp)
    }
    
    pub(crate) fn apply(&self, txn: &ReadTransaction<'static>, coll: &Collection) -> Result<Selection> {
        match self {
            Filter::Cond(cond) => {
                use self::Cond::*;
                Ok(match cond {
                    Not(filter) => !filter.apply(txn, coll)?,
                    And(filters) => {
                        let mut res = !Selection::default(); // universe
                        for filter in filters {
                            res = res & filter.apply(txn, coll)?;
                        }
                        res
                    },
                    Or(filters) => {
                        let mut res = Selection::default(); // empty
                        for filter in filters {
                            res = res | filter.apply(txn, coll)?;
                        }
                        res
                    },
                })
            },
            Filter::Comp(path, comp) => {
                let index = coll.req_index(path)?;
                let access = txn.access();
                use self::Comp::*;
                Ok(match comp {
                    Eq(val) => Selection::new(index.query_set(&txn, &access, once(val))?, false),
                    In(vals) => Selection::new(index.query_set(&txn, &access, vals.iter())?, false),
                    Gt(val) => Selection::new(index.query_range(&txn, &access, Some((val, false)), None)?, false),
                    Ge(val) => Selection::new(index.query_range(&txn, &access, Some((val, true)), None)?, false),
                    Lt(val) => Selection::new(index.query_range(&txn, &access, None, Some((val, false)))?, false),
                    Le(val) => Selection::new(index.query_range(&txn, &access, None, Some((val, true)))?, false),
                    Bw(val1, inc1, val2, inc2) => Selection::new(index.query_range(&txn, &access, Some((val1, *inc1)), Some((val2, *inc2)))?, false),
                    Has => Selection::new(index.query_range(&txn, &access, None, None)?, false),
                })
            },
        }
    }
}

/// The kind ot order
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OrderKind {
    /// Ascending ordering
    #[serde(rename="$asc")]
    Asc,
    /// Descending ordering
    #[serde(rename="$desc")]
    Desc,
}

impl Default for OrderKind {
    fn default() -> Self { OrderKind::Asc }
}

/// Ordering operator
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Order {
    /// Order by primary key/identifier of document
    ///
    /// This is default ordering
    ///
    Primary(OrderKind),

    /// Order by specified indexed field
    #[serde(with = "order")]
    Field(Identifier, OrderKind),
}

impl Default for Order {
    fn default() -> Self { Order::Primary(OrderKind::default()) }
}

impl Order {
    pub fn primary(kind: OrderKind) -> Self {
        Order::Primary(kind)
    }

    pub fn field<F: Into<Identifier>>(field: F, kind: OrderKind) -> Self {
        Order::Field(field.into(), kind)
    }
}

mod comp {
    use super::{Identifier, Comp};
    use std::collections::HashMap;
    use serde::{Serializer, Deserializer, Deserialize, de::{self}, ser::{SerializeMap}};
    
    pub fn serialize<S: Serializer>(field: &Identifier, op: &Comp, serializer: S) -> Result<S::Ok, S::Error> {
        let mut map = serializer.serialize_map(Some(1))?;
        map.serialize_entry(&field, &op)?;
        map.end()
    }
    
    pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<(Identifier, Comp), D::Error> {
        let map: HashMap<String, Comp> = HashMap::deserialize(deserializer)?;
        let mut it = map.into_iter();
        match (it.next(), it.next()) {
            (Some((field, op)), None) => Ok((field.into(), op)),
            _ => Err(de::Error::custom("Not a comp op"))
        }
    }
}

mod order {
    use super::{Identifier, OrderKind};
    use std::collections::HashMap;
    use serde::{Serializer, Deserializer, Deserialize, de::{self}, ser::{SerializeMap}};
    
    pub fn serialize<S: Serializer>(field: &Identifier, op: &OrderKind, serializer: S) -> Result<S::Ok, S::Error> {
        let mut map = serializer.serialize_map(Some(1))?;
        map.serialize_entry(&field, &op)?;
        map.end()
    }
    
    pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<(Identifier, OrderKind), D::Error> {
        let map: HashMap<String, OrderKind> = HashMap::deserialize(deserializer)?;
        let mut it = map.into_iter();
        match (it.next(), it.next()) {
            (Some((field, op)), None) => Ok((field.into(), op)),
            _ => Err(de::Error::custom("Not an order kind"))
        }
    }
}

#[cfg(test)]
mod test {
    use super::{Filter, Comp, Cond, KeyData, Order, OrderKind};
    use serde_json::{from_str, to_string, Value};

    #[test]
    fn parse_comp_eq() {
        test_parse!(Filter, json!({ "field": { "$eq": 0 } }),
                    Filter::Comp("field".into(),
                                 Comp::Eq(KeyData::Int(0))
                    ));
        test_parse!(Filter, json!({ "name": { "$eq": "vlada" } }),
                    Filter::Comp("name".into(),
                                 Comp::Eq(KeyData::String("vlada".into()))
                    ));
    }

    #[test]
    fn build_comp_eq() {
        test_build!(Filter::Comp("field".into(),
                                 Comp::Eq(KeyData::Int(0))),
                    json!({ "field": { "$eq": 0 } }));
        test_build!(Filter::Comp("name".into(),
                                 Comp::Eq(KeyData::String("vlada".into()))),
                    json!({ "name": { "$eq": "vlada" } }));
    }

    #[test]
    fn parse_cond_not() {
        test_parse!(Filter, json!({ "$not": { "a":{ "$gt": 9 } } }),
                    Filter::Cond(Cond::Not(
                        Box::new(Filter::Comp("a".into(), Comp::Gt(KeyData::Int(9)))),
                    )));
    }

    #[test]
    fn build_cond_not() {
        test_build!(Filter::Cond(Cond::Not(
            Box::new(Filter::Comp("a".into(), Comp::Gt(KeyData::Int(9))))
        )), json!({ "$not": { "a": { "$gt": 9 } } }));
    }

    #[test]
    fn parse_cond_and() {
        test_parse!(Filter, json!({ "$and": [ { "a": { "$eq": 11 } }, { "b": { "$lt": -1 } } ] }),
                    Filter::Cond(Cond::And(vec![
                        Filter::Comp("a".into(), Comp::Eq(KeyData::Int(11))),
                        Filter::Comp("b".into(), Comp::Lt(KeyData::Int(-1))),
                    ])));
    }

    #[test]
    fn build_cond_and() {
        test_build!(Filter::Cond(Cond::And(vec![
            Filter::Comp("a".into(), Comp::Eq(KeyData::Int(11))),
            Filter::Comp("b".into(), Comp::Lt(KeyData::Int(-1))),
        ])), json!({ "$and": [ { "a": { "$eq": 11 } }, { "b": { "$lt": -1 } } ] }));
    }

    #[test]
    fn parse_cond_or() {
        test_parse!(Filter, json!({ "$or": [ { "a": { "$eq": 11 } }, { "b": { "$lt": -1 } } ] }),
                   Filter::Cond(Cond::Or(vec![
                       Filter::Comp("a".into(), Comp::Eq(KeyData::Int(11))),
                       Filter::Comp("b".into(), Comp::Lt(KeyData::Int(-1))),
                   ])));
    }

    #[test]
    fn build_cond_or() {
        test_build!(Filter::Cond(Cond::Or(vec![
            Filter::Comp("a".into(), Comp::Eq(KeyData::Int(11))),
            Filter::Comp("b".into(), Comp::Lt(KeyData::Int(-1))),
        ])), json!({ "$or": [ { "a": { "$eq": 11 } }, { "b": { "$lt": -1 } } ] }));
    }

    #[test]
    fn parse_order_primary() {
        test_parse!(Order, json!("$asc"),
                    Order::Primary(OrderKind::Asc));
        test_parse!(Order, json!("$desc"),
                   Order::Primary(OrderKind::Desc));
    }

    #[test]
    fn build_order_primary() {
        test_build!(Order::Primary(OrderKind::Asc),
                   json!("$asc"));
        test_build!(Order::Primary(OrderKind::Desc),
                   json!("$desc"));
    }

    #[test]
    fn parse_order_field() {
        test_parse!(Order, json!({ "name": "$asc" }),
                   Order::Field("name".into(), OrderKind::Asc));
        test_parse!(Order, json!({ "time": "$desc" }),
                    Order::Field("time".into(), OrderKind::Desc));
    }

    #[test]
    fn build_order_field() {
        test_build!(Order::Field("name".into(), OrderKind::Asc),
                   json!({ "name": "$asc" }));
        test_build!(Order::Field("time".into(), OrderKind::Desc),
                   json!({ "time": "$desc" }));
    }
}
