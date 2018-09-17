/* debug helper
macro_rules! tts {
    ($($x:tt)+) => (
        vec![$(stringify!($x)),+]
    );
}
*/

#[macro_export(local_inner_macros)]
#[doc(hidden)]
macro_rules! field_name {
    ($($part:tt)+) => (
        concat!($(stringify!($part)),+)
    );
}

/// Filter construction helper macros
///
/// Usage examples:
///
/// ```ignore
/// // comparison operations
/// filter!(field == 123)
/// filter!(field.subfield != "abc")
/// filter!(field > 123)
/// filter!(field <= 456)
/// filter!(field in 123..456)   // [123 ... 456]
/// filter!(field <in> 123..456) // (123 ... 456)
/// filter!(field <in 123..456)  // (123 ... 456]
/// filter!(field in> 123..456)  // [123 ... 456)
///
/// // negate filter condition
/// filter!(! field == "abc")
/// // and filter conditions
/// filter!(field > 123 && field <= 456)
/// // or filter conditions
/// filter!(field <= 123 || field > 456)
/// ```
#[macro_export(local_inner_macros)]
macro_rules! filter {
    ($($tokens:tt)+) => ( filter_impl!(@parse_or [] $($tokens)+) );
}

#[macro_export(local_inner_macros)]
#[doc(hidden)]
macro_rules! filter_impl {
    // start || condition
    (@parse_or [ $($conds:tt)* ] $token:tt $($tokens:tt)*) => (
        filter_impl!(@parse_or_cond [ $($conds)* ] [ $token ] $($tokens)*)
    );
    // end || condition
    (@parse_or [ $($conds:tt)+ ]) => (
        filter_impl!(@process_or $($conds)+)
    );
    
    // end operand of || condition
    (@parse_or_cond [ $($conds:tt)* ] [ $($cond:tt)+ ] || $($tokens:tt)*) => (
        filter_impl!(@parse_or [ $($conds)* [ $($cond)+ ] ] $($tokens)*)
    );
    // end operand of || condition
    (@parse_or_cond [ $($conds:tt)* ] [ $($cond:tt)+ ]) => (
        filter_impl!(@parse_or [ $($conds)* [ $($cond)+ ] ])
    );
    // add token to current operand of || condition
    (@parse_or_cond [ $($conds:tt)* ] [ $($cond:tt)+ ] $token:tt $($tokens:tt)*) => (
        filter_impl!(@parse_or_cond [ $($conds)* ] [ $($cond)+ $token ] $($tokens)*)
    );

    // process single || condition
    (@process_or [ $($tokens:tt)+ ]) => (
        filter_impl!(@parse_and [] $($tokens)+)
    );
    // process multiple || conditions
    (@process_or $($cond:tt)+) => (
        $crate::Filter::Cond($crate::Cond::Or(vec![$(filter_impl!(@parse_and_wrapped $cond)),+]))
    );
    
    (@parse_and_wrapped [ $($tokens:tt)+ ]) => (
        filter_impl!(@parse_and [] $($tokens)+)
    );

    // start && condition
    (@parse_and [ $($conds:tt)* ] $token:tt $($tokens:tt)*) => (
        filter_impl!(@parse_and_cond [ $($conds)* ] [ $token ] $($tokens)*)
    );
    // end && condition
    (@parse_and [ $($conds:tt)+ ]) => (
        filter_impl!(@process_and $($conds)+)
    );

    // end operand of && condition
    (@parse_and_cond [ $($conds:tt)* ] [ $($cond:tt)+ ] && $($tokens:tt)*) => (
        filter_impl!(@parse_and [ $($conds)* [ $($cond)+ ] ] $($tokens)*)
    );
    // end operand of && condition
    (@parse_and_cond [ $($conds:tt)* ] [ $($cond:tt)+ ]) => (
        filter_impl!(@parse_and [ $($conds)* [ $($cond)+ ] ])
    );
    // add token to current operand of && condition
    (@parse_and_cond [ $($conds:tt)* ] [ $($cond:tt)+ ] $token:tt $($tokens:tt)*) => (
        filter_impl!(@parse_and_cond [ $($conds)* ] [ $($cond)+ $token ] $($tokens)*)
    );

    // process single && condition
    (@process_and [ $($tokens:tt)+ ]) => (
        filter_impl!(@parse_not $($tokens)+)
    );
    // process multiple && conditions
    (@process_and $($cond:tt)+) => (
        $crate::Filter::Cond($crate::Cond::And(vec![$(filter_impl!(@parse_not_wrapped $cond)),+]))
    );
    
    (@parse_not_wrapped [ $($tokens:tt)+ ]) => (
        filter_impl!(@parse_not $($tokens)+)
    );

    // parse !
    (@parse_not ! $($tokens:tt)+) => (
        $crate::Filter::Cond($crate::Cond::Not(Box::new(filter_impl!(@parse_nested $($tokens)+))))
    );

    // parse !!
    (@parse_not $($tokens:tt)+) => (
        filter_impl!(@parse_nested $($tokens)+)
    );

    // parse ()-enclosed sub-expressions
    (@parse_nested ( $($tokens:tt)+ )) => (
        filter_impl!(@parse_or [] $($tokens)+)
    );

    // parse expression
    (@parse_nested $($tokens:tt)+) => (
        filter_impl!(@parse_comp $($tokens)+)
    );

    // equal
    (@parse_comp $($field:ident).+ == $value:expr) => (
        $crate::Filter::comp(field_name!($($field).+), $crate::Comp::Eq($crate::KeyData::from($value)))
    );
    // ! equal
    (@parse_comp $($field:ident).+ != $value:expr) => (
        $crate::Filter::cond($crate::Cond::Not(Box::new($crate::Filter::comp(field_name!($($field).+), $crate::Comp::Eq($crate::KeyData::from($value))))))
    );
    // in set (one of)
    (@parse_comp $($field:ident).+ in [$($value:expr),*]) => (
        $crate::Filter::comp(field_name!($($field).+), $crate::Comp::In(vec![$($crate::KeyData::from($value)),*]))
    );

    // less than
    (@parse_comp $($field:ident).+ < $value:expr) => (
        $crate::Filter::comp(field_name!($($field).+), $crate::Comp::Lt($crate::KeyData::from($value)))
    );
    // less than or equal
    (@parse_comp $($field:ident).+ <= $value:expr) => (
        $crate::Filter::comp(field_name!($($field).+), $crate::Comp::Le($crate::KeyData::from($value)))
    );

    // greater than
    (@parse_comp $($field:ident).+ > $value:expr) => (
        $crate::Filter::comp(field_name!($($field).+), $crate::Comp::Gt($crate::KeyData::from($value)))
    );
    // greater than or equal
    (@parse_comp $($field:ident).+ >= $value:expr) => (
        $crate::Filter::comp(field_name!($($field).+), $crate::Comp::Ge($crate::KeyData::from($value)))
    );

    // in bounded range
    (@parse_comp $($field:ident).+ in $range:expr) => (
        $crate::Filter::comp(field_name!($($field).+), $crate::Comp::Bw($crate::KeyData::from($range.start), true, $crate::KeyData::from($range.end), true))
    );

    // in bounded range excluding bounds
    (@parse_comp $($field:ident).+ <in> $range:expr) => (
        $crate::Filter::comp(field_name!($($field).+), $crate::Comp::Bw($crate::KeyData::from($range.start), false, $crate::KeyData::from($range.end), false))
    );

    // in bounded range excluding start (left) bound
    (@parse_comp $($field:ident).+ <in $range:expr) => (
        $crate::Filter::comp(field_name!($($field).+), $crate::Comp::Bw($crate::KeyData::from($range.start), false, $crate::KeyData::from($range.end), true))
    );

    // in bounded range excluding end (right) bound
    (@parse_comp $($field:ident).+ in> $range:expr) => (
        $crate::Filter::comp(field_name!($($field).+), $crate::Comp::Bw($crate::KeyData::from($range.start), true, $crate::KeyData::from($range.end), false))
    );

    // has value (field exists or not null)
    (@parse_comp $($field:ident).+ ?) => (
        $crate::Filter::comp(field_name!($($field).+), $crate::Comp::Has)
    );
}

/// Order construction helper macros
///
/// Usage examples:
///
/// ```ignore
/// // ascending ordering by primary key
/// order!(>)
/// order!(v)
/// // descending ordering by primary key
/// order!(<)
/// order!(^)
/// // ascending ordering by field
/// order!(field >)
/// order!(field v)
/// // descending ordering by other.field
/// order!(other.field <)
/// order!(other.field ^)
/// ```
#[macro_export(local_inner_macros)]
macro_rules! order {
    ($($tokens:tt)+) => ( order_impl!($($tokens)*) );
}

#[macro_export(local_inner_macros)]
#[doc(hidden)]
macro_rules! order_impl {
    (>) => (
        $crate::Order::primary($crate::OrderKind::Asc)
    );
    (<) => (
        $crate::Order::primary($crate::OrderKind::Desc)
    );
    (v) => (
        $crate::Order::primary($crate::OrderKind::Asc)
    );
    (^) => (
        $crate::Order::primary($crate::OrderKind::Desc)
    );
    ($($field:ident).+ >) => (
        $crate::Order::field(field_name!($($field).+), $crate::OrderKind::Asc)
    );
    ($($field:ident).+ <) => (
        $crate::Order::field(field_name!($($field).+), $crate::OrderKind::Desc)
    );
    ($($field:ident).+ v) => (
        $crate::Order::field(field_name!($($field).+), $crate::OrderKind::Asc)
    );
    ($($field:ident).+ ^) => (
        $crate::Order::field(field_name!($($field).+), $crate::OrderKind::Desc)
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
        $m.add(field_name!($($field).+), $crate::Action::Set($crate::to_value($val).unwrap()))
    );
    (@process_action $m:ident - $($field:ident).+) => (
        $m.add(field_name!($($field).+), $crate::Action::Delete)
    );
    (@process_action $m:ident $($field:ident).+ += $val:expr) => (
        $m.add(field_name!($($field).+), $crate::Action::Add($crate::to_value($val).unwrap()))
    );
    (@process_action $m:ident $($field:ident).+ -= $val:expr) => (
        $m.add(field_name!($($field).+), $crate::Action::Sub($crate::to_value($val).unwrap()))
    );
    (@process_action $m:ident $($field:ident).+ *= $val:expr) => (
        $m.add(field_name!($($field).+), $crate::Action::Mul($crate::to_value($val).unwrap()))
    );
    (@process_action $m:ident $($field:ident).+ /= $val:expr) => (
        $m.add(field_name!($($field).+), $crate::Action::Div($crate::to_value($val).unwrap()))
    );
    (@process_action $m:ident ! $($field:ident).+) => (
        $m.add(field_name!($($field).+), $crate::Action::Toggle)
    );
    // splice helper
    (@add_splice $m:ident $($field:ident).+ [ - $start:tt .. - $delete:tt ] $($insert:tt)*) => (
        $m.add(field_name!($($field).+), $crate::Action::Splice(-$start, -$delete, modify_impl!(@insert_splice $($insert)*)))
    );
    (@add_splice $m:ident $($field:ident).+ [ - $start:tt .. $delete:tt ] $($insert:tt)*) => (
        $m.add(field_name!($($field).+), $crate::Action::Splice(-$start, $delete, modify_impl!(@insert_splice $($insert)*)))
    );
    (@add_splice $m:ident $($field:ident).+ [ $start:tt .. - $delete:tt ] $($insert:tt)*) => (
        $m.add(field_name!($($field).+), $crate::Action::Splice($start, -$delete, modify_impl!(@insert_splice $($insert)*)))
    );
    (@add_splice $m:ident $($field:ident).+ [ $start:tt .. $delete:tt ] $($insert:tt)*) => (
        $m.add(field_name!($($field).+), $crate::Action::Splice($start, $delete, modify_impl!(@insert_splice $($insert)*)))
    );
    (@add_splice $m:ident $($field:ident).+ [ - $start:tt .. ] $($insert:tt)*) => (
        $m.add(field_name!($($field).+), $crate::Action::Splice(-$start, -1, modify_impl!(@insert_splice $($insert)*)))
    );
    (@add_splice $m:ident $($field:ident).+ [ $start:tt .. ] $($insert:tt)*) => (
        $m.add(field_name!($($field).+), $crate::Action::Splice($start, -1, modify_impl!(@insert_splice $($insert)*)))
    );
    (@add_splice $m:ident $($field:ident).+ [ .. - $delete:tt ] $($insert:tt)*) => (
        $m.add(field_name!($($field).+), $crate::Action::Splice(0, -$delete, modify_impl!(@insert_splice $($insert)*)))
    );
    (@add_splice $m:ident $($field:ident).+ [ .. $delete:tt ] $($insert:tt)*) => (
        $m.add(field_name!($($field).+), $crate::Action::Splice(0, $delete, modify_impl!(@insert_splice $($insert)*)))
    );
    (@add_splice $m:ident $($field:ident).+ [ .. ] $($insert:tt)*) => (
        $m.add(field_name!($($field).+), $crate::Action::Splice(0, -1, modify_impl!(@insert_splice $($insert)*)))
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
        $m.add(field_name!($($field).+), $crate::Action::Merge($crate::to_value(json!({ $($obj)+ })).unwrap()))
    );
    (@process_action $m:ident $($field:ident).+ ~= $obj:expr) => (
        $m.add(field_name!($($field).+), $crate::Action::Merge($crate::to_value($obj).unwrap()))
    );
    // string replace
    (@process_action $m:ident $($field:ident).+ ~= $pat:tt $sub:expr) => (
        $m.add(field_name!($($field).+), $crate::Action::Replace($crate::WrappedRegex($pat.parse().unwrap()), String::from($sub)))
    );
    /*(@process_action $m:ident $($any:tt)+) => (
        println!("!! {:?}", tts!($($any)+))
    );*/
}



#[cfg(test)]
mod test {
    mod filter {
        use serde_json::from_value;
        
        #[test]
        fn comp_eq() {
            assert_eq!(filter!(f == 123), json_val!({ "f": { "$eq": 123 } }));
            assert_eq!(filter!(f != 123), json_val!({ "$not": { "f": { "$eq": 123 } } }));
            assert_eq!(filter!(f != "abc"), json_val!({ "$not": { "f": { "$eq": "abc" } } }));
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

            assert_eq!(filter!(f in -12..-34), json_val!({ "f": { "$bw": [-12, true, -34, true] } }));
            assert_eq!(filter!(f <in> -12..-34), json_val!({ "f": { "$bw": [-12, false, -34, false] } }));
            assert_eq!(filter!(f <in -12..-34), json_val!({ "f": { "$bw": [-12, false, -34, true] } }));
            assert_eq!(filter!(f in> -12..-34), json_val!({ "f": { "$bw": [-12, true, -34, false] } }));
        }

        #[test]
        fn comp_has() {
            assert_eq!(filter!(f?), json_val!({ "f": "$has" }));
        }

        #[test]
        fn cond_not() {
            assert_eq!(filter!(!a?), json_val!({ "$not": { "a": "$has" } }));
        }

        #[test]
        fn cond_and() {
            assert_eq!(filter!(a == 3 && b >= 1), json_val!({ "$and": [ { "a": { "$eq": 3 } }, { "b": { "$ge": 1 } } ] }));
        }

        #[test]
        fn cond_or() {
            assert_eq!(filter!(a == "abc" || b <in> 12..34), json_val!({ "$or": [ { "a": { "$eq": "abc" } }, { "b": { "$bw": [12, false, 34, false] } } ] }));
        }

        #[test]
        fn cond_not_and() {
            assert_eq!(filter!(!(a == "abc" && b <in> 12..34)), json_val!({ "$not": { "$and": [ { "a": { "$eq": "abc" } }, { "b": { "$bw": [12, false, 34, false] } } ] } }));
        }

        #[test]
        fn cond_and_not() {
            assert_eq!(filter!(a != "abc" && !(b <in> 12..34)), json_val!({ "$and": [ { "$not": { "a": { "$eq": "abc" } } }, { "$not": { "b": { "$bw": [12, false, 34, false] } } } ] }));
        }

        #[test]
        fn cond_and_or() {
            let b_edge = 10;
            
            assert_eq!(filter!(a in [1, 2, 3] && (b > b_edge || b < -b_edge)), json_val!({ "$and": [ { "a": { "$in": [1, 2, 3] } }, { "$or": [ { "b": { "$gt": 10 } }, { "b": { "$lt": -10 } } ] } ] }));
        }

        #[test]
        fn comp_sub_fields() {
            assert_eq!(filter!(a.b.c == 1), json_val!({ "a.b.c": { "$eq": 1 } }));
        }

        #[test]
        fn or_nested_and() {
            assert_eq!(filter!(a == 1 || !b == "abc" && c < 5),
                       json_val!({ "$or": [
                           { "a": { "$eq": 1 } },
                           { "$and": [
                               { "$not": { "b": { "$eq": "abc" } } },
                               { "c": { "$lt": 5 } }
                           ] }
                       ] }));
            assert_eq!(filter!(a == 1 && !b == "abc" || c < 5),
                       json_val!({ "$or": [
                           { "$and": [
                               { "a": { "$eq": 1 } },
                               { "$not": { "b": { "$eq": "abc" } } }
                           ] },
                           { "c": { "$lt": 5 } }
                       ] }));
        }

        #[test]
        fn and_nested_or() {
            assert_eq!(filter!((a == 1 || !b == "abc") && c < 5),
                       json_val!({ "$and": [
                           { "$or": [
                               { "a": { "$eq": 1 } },
                               { "$not": { "b": { "$eq": "abc" } } }
                           ] },
                           { "c": { "$lt": 5 } }
                       ] }));
            assert_eq!(filter!(a == 1 && (!b == "abc" || c < 5)),
                       json_val!({ "$and": [
                           { "a": { "$eq": 1 } },
                           { "$or": [
                               { "$not": { "b": { "$eq": "abc" } } },
                               { "c": { "$lt": 5 } }
                           ] }
                       ] }));
        }
    }

    mod order {
        use serde_json::from_value;
        
        #[test]
        fn primary_asc() {
            assert_eq!(order!(>), json_val!("$asc"));
            assert_eq!(order!(v), json_val!("$asc"));
        }
        
        #[test]
        fn primary_desc() {
            assert_eq!(order!(<), json_val!("$desc"));
            assert_eq!(order!(^), json_val!("$desc"));
        }
        
        #[test]
        fn field_asc() {
            assert_eq!(order!(field >), json_val!({ "field": "$asc" }));
            assert_eq!(order!(field v), json_val!({ "field": "$asc" }));
        }
        
        #[test]
        fn field_desc() {
            assert_eq!(order!(a.b.c <), json_val!({ "a.b.c": "$desc" }));
            assert_eq!(order!(a.b.c ^), json_val!({ "a.b.c": "$desc" }));
        }
    }

    mod modify {
        use serde_json::from_value;
        
        #[test]
        fn set() {
            assert_eq!(modify!(a = 1u32), json_val!({ "a": { "$set": 1 } }));
            assert_eq!(modify!(a = 123u32, b.c = "abc"), json_val!({ "a": { "$set": 123 }, "b.c": { "$set": "abc" } }));
            assert_eq!(modify!(
                a = 123u32;
                b.c = "abc";
            ), json_val!({ "a": { "$set": 123 }, "b.c": { "$set": "abc" } }));
        }
        
        #[test]
        fn delete() {
            assert_eq!(modify!(-field), json_val!({ "field": "$delete" }));
            assert_eq!(modify!(-field, -other.field), json_val!({ "field": "$delete", "other.field": "$delete" }));
        }
        
        #[test]
        fn add() {
            assert_eq!(modify!(field += 123u32), json_val!({ "field": { "$add": 123 } }));
            assert_eq!(modify!(field += 123u32, other.field += "abc"), json_val!({ "field": { "$add": 123 }, "other.field": { "$add": "abc" } }));
        }

        #[test]
        fn sub() {
            assert_eq!(modify!(field -= 123u32), json_val!({ "field": { "$sub": 123 } }));
            assert_eq!(modify!(field -= 123u32, other.field -= "abc"), json_val!({ "field": { "$sub": 123 }, "other.field": { "$sub": "abc" } }));
        }
        
        #[test]
        fn mul() {
            assert_eq!(modify!(field *= 123u32), json_val!({ "field": { "$mul": 123 } }));
            assert_eq!(modify!(field *= 123u32, other.field *= "abc"), json_val!({ "field": { "$mul": 123 }, "other.field": { "$mul": "abc" } }));
        }

        #[test]
        fn div() {
            assert_eq!(modify!(field /= 123u32), json_val!({ "field": { "$div": 123 } }));
            assert_eq!(modify!(field /= 123u32, other.field /= "abc"), json_val!({ "field": { "$div": 123 }, "other.field": { "$div": "abc" } }));
        }

        #[test]
        fn toggle() {
            assert_eq!(modify!(!field), json_val!({ "field": "$toggle" }));
            assert_eq!(modify!(!field, !other.field), json_val!({ "field": "$toggle", "other.field": "$toggle" }));
            assert_eq!(modify!(!field; !field), json_val!({ "field": ["$toggle", "$toggle"] }));
        }

        #[test]
        fn replace() {
            assert_eq!(modify!(field ~= "abc" "def"), json_val!({ "field": { "$replace": ["abc", "def"] } }));
            assert_eq!(modify!(field ~= "abc" "def", other.field ~= "april" "may"), json_val!({ "field": { "$replace": ["abc", "def"] }, "other.field": { "$replace": ["april", "may"] } }));
        }
        
        #[test]
        fn splice() {
            assert_eq!(modify!(-field[1..2]), json_val!({ "field": { "$splice": [1, 2] } }));
            
            assert_eq!(modify!(field[1..2] = ["a", "b", "c"]), json_val!({ "field": { "$splice": [1, 2, "a", "b", "c"] } }));
        
            let ins = [1u8, 2, 3];
            assert_eq!(modify!(other.field[-1..0] = ins), json_val!({ "other.field": { "$splice": [-1, 0, 1, 2, 3] } }));
            
            assert_eq!(modify!(-field[..]), json_val!({ "field": { "$splice": [0, -1] } }));
        }

        #[test]
        fn merge() {
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
}
