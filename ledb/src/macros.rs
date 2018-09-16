macro_rules! tts {
    ($($x:tt)+) => (
        vec![$(stringify!($x)),+]
    );
}

#[macro_export(local_inner_macros)]
#[doc(hidden)]
macro_rules! field_name_str {
    ($($part:tt)+) => (
        concat!($(stringify!($part)),+)
    );
}

#[macro_export(local_inner_macros)]
#[doc(hidden)]
macro_rules! field_name {
    ($($part:tt)+) => (
        concat!($(stringify!($part)),+).into()
    );
}

/// Filter construction helper macros
///
/// Usage examples:
///
/// ```ignore
/// filter!(field == 123)
/// filter!(field.subfield != "abc")
/// filter!(field > 123)
/// filter!(field <= 456)
/// filter!(field in 123..456)   // [123 ... 456]
/// filter!(field <in> 123..456) // (123 ... 456)
/// filter!(field <in 123..456)  // (123 ... 456]
/// filter!(field in> 123..456)  // [123 ... 456)
/// filter!(![field == "abc"])
/// filter!([field > 123] && [field <= 456])
/// filter!([field == 123] || [other.field == 456])
/// ```
#[macro_export(local_inner_macros)]
macro_rules! filter {
    ($($tokens:tt)+) => ( filter_impl!($($tokens)*) );
}

#[macro_export(local_inner_macros)]
#[doc(hidden)]
macro_rules! filter_impl {
    ($($field:ident).+ == $value:expr) => (
        $crate::Filter::Comp(field_name!($($field).+), $crate::Comp::Eq($crate::KeyData::from($value)))
    );
    
    ($($field:ident).+ = $value:expr) => (
        $crate::Filter::Comp(field_name!($($field).+), $crate::Comp::Eq($crate::KeyData::from($value)))
    );

    ($($field:ident).+ !== $value:expr) => (
        $crate::Filter::Cond($crate::Cond::Not(Box::new($crate::Filter::Comp(field_name!($($field).+), $crate::Comp::Eq($crate::KeyData::from($value))))))
    );

    ($($field:ident).+ != $value:expr) => (
        $crate::Filter::Cond($crate::Cond::Not(Box::new($crate::Filter::Comp(field_name!($($field).+), $crate::Comp::Eq($crate::KeyData::from($value))))))
    );

    ($($field:ident).+ in [$($value:expr),*]) => (
        $crate::Filter::Comp(field_name!($($field).+), $crate::Comp::In(vec![$($crate::KeyData::from($value)),*]))
    );

    ($($field:ident).+ < $value:expr) => (
        $crate::Filter::Comp(field_name!($($field).+), $crate::Comp::Lt($crate::KeyData::from($value)))
    );

    ($($field:ident).+ <= $value:expr) => (
        $crate::Filter::Comp(field_name!($($field).+), $crate::Comp::Le($crate::KeyData::from($value)))
    );

    ($($field:ident).+ > $value:expr) => (
        $crate::Filter::Comp(field_name!($($field).+), $crate::Comp::Gt($crate::KeyData::from($value)))
    );

    ($($field:ident).+ >= $value:expr) => (
        $crate::Filter::Comp(field_name!($($field).+), $crate::Comp::Ge($crate::KeyData::from($value)))
    );

    ($($field:ident).+ in $range:expr) => (
        $crate::Filter::Comp(field_name!($($field).+), $crate::Comp::Bw($crate::KeyData::from($range.start), true, $crate::KeyData::from($range.end), true))
    );

    ($($field:ident).+ <in> $range:expr) => (
        $crate::Filter::Comp(field_name!($($field).+), $crate::Comp::Bw($crate::KeyData::from($range.start), false, $crate::KeyData::from($range.end), false))
    );

    ($($field:ident).+ <in $range:expr) => (
        $crate::Filter::Comp(field_name!($($field).+), $crate::Comp::Bw($crate::KeyData::from($range.start), false, $crate::KeyData::from($range.end), true))
    );

    ($($field:ident).+ in> $range:expr) => (
        $crate::Filter::Comp(field_name!($($field).+), $crate::Comp::Bw($crate::KeyData::from($range.start), true, $crate::KeyData::from($range.end), false))
    );

    ($($field:ident).+ ?) => (
        $crate::Filter::Comp(field_name!($($field).+), $crate::Comp::Has)
    );

    (![$($cond:tt)+]) => (
        $crate::Filter::Cond($crate::Cond::Not(Box::new(filter_impl!($($cond)+))))
    );
    
    ($([$($cond:tt)+])&&+) => (
        $crate::Filter::Cond($crate::Cond::And(vec![$(filter_impl!($($cond)+)),*]))
    );

    ($([$($cond:tt)+])||+) => (
        $crate::Filter::Cond($crate::Cond::Or(vec![$(filter_impl!($($cond)+)),*]))
    );
}

/// Order construction helper macros
///
/// Usage examples:
///
/// ```ignore
/// order!(>) // ascending ordering by primary key
/// order!(<) // descending ordering by primary key
/// order!(field >) // ascending ordering by field
/// order!(other.field <) // descending ordering by other.field
/// ```
#[macro_export(local_inner_macros)]
macro_rules! order {
    ($($tokens:tt)+) => ( order_impl!($($tokens)*) );
}

#[macro_export(local_inner_macros)]
#[doc(hidden)]
macro_rules! order_impl {
    (>) => (
        $crate::Order::Primary($crate::OrderKind::Asc)
    );
    (<) => (
        $crate::Order::Primary($crate::OrderKind::Desc)
    );
    ($($field:ident).+ >) => (
        $crate::Order::Field(field_name!($($field).+), $crate::OrderKind::Asc)
    );
    ($($field:ident).+ <) => (
        $crate::Order::Field(field_name!($($field).+), $crate::OrderKind::Desc)
    );
}

/// Modifier construction helper macros
///
/// Usage examples:
///
/// ```ignore
/// // set single fields
/// modify!(field = 123)
/// modify!(other.field = "abc")
///
/// // set multiple fields
/// modify!(
///     field = 1;
///     other.field = "abc";
/// )
///
/// // numeric operations
/// modify!(field += 1) // add value to field
/// modify!(field -= 1) // substract value from field
/// modify!(field *= 1) // multiply field to value
/// modify!(field /= 1) // divide field to value
///
/// modify!(- field) // remove field
/// modify!(! field) // toggle boolean field
///
/// modify!(str += "addon") // append piece to string
/// modify!(str ~= "abc" "def") // regexp replace
///
/// // modify array as list
/// modify!(list[0..0] = [1, 2, 3]) // prepend to array
/// modify!(list[-1..0] = [1, 2, 3]) // append to array
/// modify!(- list[1..2]) // remove from array
/// modify!(list[1..2] = [1, 2, 3]) // splice array
///
/// // modify array as set
/// modify!(set += [1, 2, 3]) // add elements
/// modify!(set -= [4, 5, 6]) // remove elements
///
/// // merge an object
/// modify!(obj ~= { a: true, b: "abc", c: 123 })
/// modify!(obj ~= extra)
/// ```
///
/// The negative range value means reverse position in array:
/// * -1 the end of an array
/// * -2 the last element
/// * -3 the element before the last
/// ...and so on.
#[macro_export(local_inner_macros)]
macro_rules! modify {
    ($($tokens:tt)+) => ( modify_impl!(@parse [] $($tokens)+) );
}

#[macro_export(local_inner_macros)]
#[doc(hidden)]
macro_rules! modify_impl {
    // parsing
    (@parse [ $($actions:tt)* ] , $($tokens:tt)*) => ( // skip commas at top level
        modify_impl!(@parse [ $($actions)* ] $($tokens)*)
    );
    (@parse [ $($actions:tt)* ] ; $($tokens:tt)*) => ( // skip semicolons at top level
        modify_impl!(@parse [ $($actions)* ] $($tokens)*)
    );
    (@parse [ $($actions:tt)* ] ) => ( // goto processing
        modify_impl!(@process $($actions)*)
    );
    (@parse [ $($actions:tt)* ] $token:tt $($tokens:tt)*) => ( // start action parsing
        modify_impl!(@parse_action [ $($actions)* ] { $token } $($tokens)*)
    );
    // parsing action
    (@parse_action [ $($actions:tt)* ] { $($action:tt)+ } , $($tokens:tt)*) => ( // end action parsing
        modify_impl!(@parse [ $($actions)* { $($action)+ } ] $($tokens)*)
    );
    (@parse_action [ $($actions:tt)* ] { $($action:tt)+ } ; $($tokens:tt)*) => ( // end action parsing
        modify_impl!(@parse [ $($actions)* { $($action)+ } ] $($tokens)*)
    );
    (@parse_action [ $($actions:tt)* ] { $($action:tt)+ } ) => ( // end action parsing
        modify_impl!(@parse [ $($actions)* { $($action)+ } ])
    );
    (@parse_action [ $($actions:tt)* ] { $($action:tt)+ } $token:tt $($tokens:tt)*) => ( // action parsing
        modify_impl!(@parse_action [ $($actions)* ] { $($action)+ $token } $($tokens)*)
    );
    // processing
    (@process $($actions:tt)*) => ({ // top level processing
        let mut m = $crate::Modify::default();
        modify_impl!(@process_actions m $($actions)*);
        m
    });
    (@process_actions $m:ident { $($action:tt)+ } $($actions:tt)*) => (
        modify_impl!(@process_action $m $($action)+);
        modify_impl!(@process_actions $m $($actions)*)
    );
    (@process_actions $m:ident ) => (
    );
    // processing action
    (@process_action $m:ident $($field:ident).+ = $val:expr) => (
        $m.add(field_name_str!($($field).+), $crate::Action::Set($crate::to_value($val).unwrap()))
    );
    (@process_action $m:ident - $($field:ident).+) => (
        $m.add(field_name_str!($($field).+), $crate::Action::Delete)
    );
    (@process_action $m:ident $($field:ident).+ += $val:expr) => (
        $m.add(field_name_str!($($field).+), $crate::Action::Add($crate::to_value($val).unwrap()))
    );
    (@process_action $m:ident $($field:ident).+ -= $val:expr) => (
        $m.add(field_name_str!($($field).+), $crate::Action::Sub($crate::to_value($val).unwrap()))
    );
    (@process_action $m:ident $($field:ident).+ *= $val:expr) => (
        $m.add(field_name_str!($($field).+), $crate::Action::Mul($crate::to_value($val).unwrap()))
    );
    (@process_action $m:ident $($field:ident).+ /= $val:expr) => (
        $m.add(field_name_str!($($field).+), $crate::Action::Div($crate::to_value($val).unwrap()))
    );
    (@process_action $m:ident ! $($field:ident).+) => (
        $m.add(field_name_str!($($field).+), $crate::Action::Toggle)
    );
    // splice helper
    (@add_splice $m:ident $($field:ident).+ [ - $start:tt .. - $delete:tt ] $($insert:tt)*) => (
        $m.add(field_name_str!($($field).+), $crate::Action::Splice(-$start, -$delete, modify_impl!(@insert_splice $($insert)*)))
    );
    (@add_splice $m:ident $($field:ident).+ [ - $start:tt .. $delete:tt ] $($insert:tt)*) => (
        $m.add(field_name_str!($($field).+), $crate::Action::Splice(-$start, $delete, modify_impl!(@insert_splice $($insert)*)))
    );
    (@add_splice $m:ident $($field:ident).+ [ $start:tt .. - $delete:tt ] $($insert:tt)*) => (
        $m.add(field_name_str!($($field).+), $crate::Action::Splice($start, -$delete, modify_impl!(@insert_splice $($insert)*)))
    );
    (@add_splice $m:ident $($field:ident).+ [ $start:tt .. $delete:tt ] $($insert:tt)*) => (
        $m.add(field_name_str!($($field).+), $crate::Action::Splice($start, $delete, modify_impl!(@insert_splice $($insert)*)))
    );
    (@add_splice $m:ident $($field:ident).+ [ - $start:tt .. ] $($insert:tt)*) => (
        $m.add(field_name_str!($($field).+), $crate::Action::Splice(-$start, -1, modify_impl!(@insert_splice $($insert)*)))
    );
    (@add_splice $m:ident $($field:ident).+ [ $start:tt .. ] $($insert:tt)*) => (
        $m.add(field_name_str!($($field).+), $crate::Action::Splice($start, -1, modify_impl!(@insert_splice $($insert)*)))
    );
    (@add_splice $m:ident $($field:ident).+ [ .. - $delete:tt ] $($insert:tt)*) => (
        $m.add(field_name_str!($($field).+), $crate::Action::Splice(0, -$delete, modify_impl!(@insert_splice $($insert)*)))
    );
    (@add_splice $m:ident $($field:ident).+ [ .. $delete:tt ] $($insert:tt)*) => (
        $m.add(field_name_str!($($field).+), $crate::Action::Splice(0, $delete, modify_impl!(@insert_splice $($insert)*)))
    );
    (@add_splice $m:ident $($field:ident).+ [ .. ] $($insert:tt)*) => (
        $m.add(field_name_str!($($field).+), $crate::Action::Splice(0, -1, modify_impl!(@insert_splice $($insert)*)))
    );
    (@insert_splice $insert:expr) => (
        $insert.iter().map(|elm| $crate::to_value(elm).unwrap()).collect()
    );
    (@insert_splice ) => (
        Vec::new()
    );
    // remove from an array
    (@process_action $m:ident - $($field:ident).+ [ $($range:tt)+ ]) => (
        modify_impl!(@add_splice $m $($field).+ [ $($range)+ ])
    );
    // splice array
    (@process_action $m:ident $($field:ident).+ [ $($range:tt)+ ] = $insert:expr) => (
        modify_impl!(@add_splice $m $($field).+ [ $($range)+ ] $insert)
    );
    // merge object
    (@process_action $m:ident $($field:ident).+ ~= { $($obj:tt)+ }) => (
        $m.add(field_name_str!($($field).+), $crate::Action::Merge($crate::to_value(json!({ $($obj)+ })).unwrap()))
    );
    (@process_action $m:ident $($field:ident).+ ~= $obj:expr) => (
        $m.add(field_name_str!($($field).+), $crate::Action::Merge($crate::to_value($obj).unwrap()))
    );
    // string replace
    (@process_action $m:ident $($field:ident).+ ~= $pat:tt $sub:expr) => (
        $m.add(field_name_str!($($field).+), $crate::Action::Replace($crate::WrappedRegex($pat.parse().unwrap()), String::from($sub)))
    );
    /*(@process_action $m:ident $($any:tt)+) => (
        println!("!! {:?}", tts!($($any)+))
    );*/
}

#[cfg(test)]
mod test {
    use serde_json::{from_value};
    
    #[test]
    fn comp_eq() {
        assert_eq!(filter!(f == 123), json_val!({ "f": { "$eq": 123 } }));
        assert_eq!(filter!(f = 123), json_val!({ "f": { "$eq": 123 } }));
        assert_eq!(filter!(f = "abc"), json_val!({ "f": { "$eq": "abc" } }));
        assert_eq!(filter!(f != 123), json_val!({ "$not": { "f": { "$eq": 123 } } }));
        assert_eq!(filter!(f !== "abc"), json_val!({ "$not": { "f": { "$eq": "abc" } } }));
    }

    #[test]
    fn comp_in() {
        assert_eq!(filter!(f in [1, 2, 3]), json_val!({ "f": { "$in": [1, 2, 3] } }));
        assert_eq!(filter!(f in ["a", "b", "c"]), json_val!({ "f": { "$in": ["a", "b", "c"] } }));
    }

    #[test]
    fn comp_lt() {
        assert_eq!(filter!(f < 123), json_val!({ "f": { "$lt": 123 } }));
    }

    #[test]
    fn comp_le() {
        assert_eq!(filter!(f <= 123), json_val!({ "f": { "$le": 123 } }));
    }

    #[test]
    fn comp_gt() {
        assert_eq!(filter!(f > 123), json_val!({ "f": { "$gt": 123 } }));
    }

    #[test]
    fn comp_ge() {
        assert_eq!(filter!(f >= 123), json_val!({ "f": { "$ge": 123 } }));
    }

    #[test]
    fn comp_bw() {
        assert_eq!(filter!(f in 12..34), json_val!({ "f": { "$bw": [12, true, 34, true] } }));
        assert_eq!(filter!(f <in> 12..34), json_val!({ "f": { "$bw": [12, false, 34, false] } }));
        assert_eq!(filter!(f <in 12..34), json_val!({ "f": { "$bw": [12, false, 34, true] } }));
        assert_eq!(filter!(f in> 12..34), json_val!({ "f": { "$bw": [12, true, 34, false] } }));
    }

    #[test]
    fn comp_has() {
        assert_eq!(filter!(f?), json_val!({ "f": "$has" }));
    }

    #[test]
    fn cond_not() {
        assert_eq!(filter!(![a?]), json_val!({ "$not": { "a": "$has" } }));
    }

    #[test]
    fn cond_and() {
        assert_eq!(filter!([a == 3] && [b >= 1]), json_val!({ "$and": [ { "a": { "$eq": 3 } }, { "b": { "$ge": 1 } } ] }));
    }

    #[test]
    fn cond_or() {
        assert_eq!(filter!([a == "abc"] || [b <in> 12..34]), json_val!({ "$or": [ { "a": { "$eq": "abc" } }, { "b": { "$bw": [12, false, 34, false] } } ] }));
    }

    #[test]
    fn cond_not_and() {
        assert_eq!(filter!(![[a == "abc"] && [b <in> 12..34]]), json_val!({ "$not": { "$and": [ { "a": { "$eq": "abc" } }, { "b": { "$bw": [12, false, 34, false] } } ] } }));
    }

    #[test]
    fn cond_and_not() {
        assert_eq!(filter!([a != "abc"] && [![b <in> 12..34]]), json_val!({ "$and": [ { "$not": { "a": { "$eq": "abc" } } }, { "$not": { "b": { "$bw": [12, false, 34, false] } } } ] }));
    }

    #[test]
    fn cond_and_or() {
        let b_edge = 10;
        
        assert_eq!(filter!([a in [1, 2, 3]] && [[b > b_edge] || [b < -b_edge]]), json_val!({ "$and": [ { "a": { "$in": [1, 2, 3] } }, { "$or": [ { "b": { "$gt": 10 } }, { "b": { "$lt": -10 } } ] } ] }));
    }

    #[test]
    fn sub_field() {
        assert_eq!(filter!(a.b.c == 1), json_val!({ "a.b.c": { "$eq": 1 } }));
    }

    #[test]
    fn order_primary_asc() {
        assert_eq!(order!(>), json_val!("$asc"));
    }

    #[test]
    fn order_primary_desc() {
        assert_eq!(order!(<), json_val!("$desc"));
    }

    #[test]
    fn order_field_asc() {
        assert_eq!(order!(field >), json_val!({ "field": "$asc" }));
    }

    #[test]
    fn order_field_desc() {
        assert_eq!(order!(a.b.c <), json_val!({ "a.b.c": "$desc" }));
    }

    #[test]
    fn modify_set() {
        assert_eq!(modify!(a = 1u32), json_val!({ "a": { "$set": 1 } }));
        assert_eq!(modify!(a = 123u32, b.c = "abc"), json_val!({ "a": { "$set": 123 }, "b.c": { "$set": "abc" } }));
        assert_eq!(modify!(
            a = 123u32;
            b.c = "abc";
        ), json_val!({ "a": { "$set": 123 }, "b.c": { "$set": "abc" } }));
    }

    #[test]
    fn modify_delete() {
        assert_eq!(modify!(-field), json_val!({ "field": "$delete" }));
        assert_eq!(modify!(-field, -other.field), json_val!({ "field": "$delete", "other.field": "$delete" }));
    }

    #[test]
    fn modify_add() {
        assert_eq!(modify!(field += 123u32), json_val!({ "field": { "$add": 123 } }));
        assert_eq!(modify!(field += 123u32, other.field += "abc"), json_val!({ "field": { "$add": 123 }, "other.field": { "$add": "abc" } }));
    }

    #[test]
    fn modify_sub() {
        assert_eq!(modify!(field -= 123u32), json_val!({ "field": { "$sub": 123 } }));
        assert_eq!(modify!(field -= 123u32, other.field -= "abc"), json_val!({ "field": { "$sub": 123 }, "other.field": { "$sub": "abc" } }));
    }

    #[test]
    fn modify_mul() {
        assert_eq!(modify!(field *= 123u32), json_val!({ "field": { "$mul": 123 } }));
        assert_eq!(modify!(field *= 123u32, other.field *= "abc"), json_val!({ "field": { "$mul": 123 }, "other.field": { "$mul": "abc" } }));
    }

    #[test]
    fn modify_div() {
        assert_eq!(modify!(field /= 123u32), json_val!({ "field": { "$div": 123 } }));
        assert_eq!(modify!(field /= 123u32, other.field /= "abc"), json_val!({ "field": { "$div": 123 }, "other.field": { "$div": "abc" } }));
    }

    #[test]
    fn modify_toggle() {
        assert_eq!(modify!(!field), json_val!({ "field": "$toggle" }));
        assert_eq!(modify!(!field, !other.field), json_val!({ "field": "$toggle", "other.field": "$toggle" }));
        assert_eq!(modify!(!field; !field), json_val!({ "field": ["$toggle", "$toggle"] }));
    }

    #[test]
    fn modify_replace() {
        assert_eq!(modify!(field ~= "abc" "def"), json_val!({ "field": { "$replace": ["abc", "def"] } }));
        assert_eq!(modify!(field ~= "abc" "def", other.field ~= "april" "may"), json_val!({ "field": { "$replace": ["abc", "def"] }, "other.field": { "$replace": ["april", "may"] } }));
    }

    #[test]
    fn modify_splice() {
        assert_eq!(modify!(-field[1..2]), json_val!({ "field": { "$splice": [1, 2] } }));
        
        assert_eq!(modify!(field[1..2] = ["a", "b", "c"]), json_val!({ "field": { "$splice": [1, 2, "a", "b", "c"] } }));
        
        let ins = [1u8, 2, 3];
        assert_eq!(modify!(other.field[-1..0] = ins), json_val!({ "other.field": { "$splice": [-1, 0, 1, 2, 3] } }));

        assert_eq!(modify!(-field[..]), json_val!({ "field": { "$splice": [0, -1] } }));
    }

    #[test]
    fn modify_merge() {
        #[derive(Serialize, Deserialize)]
        struct Extra {
            subfield: bool,
            other: u8,
        }

        let extra = Extra { subfield: true, other: 123 };
        assert_eq!(modify!(field ~= extra), json_val!({ "field": { "$merge": { "subfield": true, "other": 123 } } }));

        assert_eq!(modify!(field ~= { "subfield": true, "other": 123 }), json_val!({ "field": { "$merge": { "subfield": true, "other": 123 } } }));
    }
}
