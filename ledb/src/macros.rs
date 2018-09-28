/// Unified query macro
///
/// ```ignore
/// #[macro_use] extern crate serde_derive;
/// #[macro_use] extern crate ledb;
/// use ledb::*;
///
/// #[derive(Serialize, Deserialize)]
/// struct MyDoc {
///   field: String,
/// }
///
/// fn main() {
///     let db_handle = Storage::open(".my_db").unwrap();
///     let my_collection = db_handle.collection("my_collection").unwrap();
///
///     // ensure index
///     assert!(
///         query!(index for my_collection
///             field Int,
///             other_field Binary,
///             field.subfield String unique,
///         ).is_ok()
///     );
///
///     // find query
///     assert!(
///         query!(find in my_collection where field == "abc").is_ok()
///     );
///
///     // find query with ascending ordering by field
///     assert!(
///         query!(find in my_collection where [field == "abc"] order by other.field).is_ok()
///     );
///
///     // find query with result document type with descending ordering by primary key
///     assert!(
///         query!(find MyDoc in my_collection where [field == "abc"] order ^).is_ok()
///     );
///
///     // update query
///     assert!(
///         query!(update in my_collection modify [field = "def"] where [field == "abc"]).is_ok()
///     );
///
///     // remove query
///     assert!(
///         query!(remove from my_collection where [field == "def"]).is_ok()
///     );
/// }
/// ```
#[macro_export]
macro_rules! query {
    // call util
    (@$util:ident $($args:tt)*) => ( _query_impl!(@$util $($args)*) );

    // make query
    ($($tokens:tt)+) => ( _query_impl!(@query _query_native, $($tokens)+) );
}

// native API output macros
#[macro_export]
#[doc(hidden)]
macro_rules! _query_native {
    (@index $coll:expr, $($indexes:tt),+) => ( $coll.set_indexes(&[$($indexes),+]) );
    (@find $type:tt, $coll:expr, $filter:expr, $order:expr) => ( $coll.find::<$type>($filter, $order) );
    (@insert $coll:expr, $doc:expr) => ( $coll.insert(&$doc) );
    (@update $coll:expr, $filter:expr, $modify:expr) => ( $coll.update($filter, $modify) );
    (@remove $coll:expr, $filter:expr) => ( $coll.remove($filter) );
}

// query DSL implementation
#[macro_export]
#[doc(hidden)]
macro_rules! _query_impl {
    // ensure index
    (@query $out:ident, index for $coll:tt $($tokens:tt)+) => ( _query_impl!(@index ($out, $coll), $($tokens)+) );

    // find with document type
    (@query $out:ident, find $type:tt in $coll:tt $($tokens:tt)*) => ( _query_impl!(@find ($out, $type, $coll), $($tokens)*) );
    // find
    (@query $out:ident, find in $coll:tt $($tokens:tt)*) => ( _query_impl!(@find ($out, _, $coll), $($tokens)*) );

    // insert
    (@query $out:ident, insert into $coll:tt $($tokens:tt)+) => ( _query_impl!(@insert ($out, $coll), $($tokens)+) );

    // update
    (@query $out:ident, update in $coll:tt $($tokens:tt)+) => ( _query_impl!(@update ($out, $coll), $($tokens)+) );

    // remove
    (@query $out:ident, remove from $coll:tt $($tokens:tt)*) => ( _query_impl!(@remove ($out, $coll), $($tokens)+) );

    //
    // Parse query
    //

    // index query
    (@index $args:tt, $($tokens:tt)+) => (
        _query_impl!(@index_fields $args, [], $($tokens)+)
    );
    // index field tuple
    (@index_field $($field:ident).+, $kind:ident, $type:ident) => (
        (_query_impl!(@field $($field).+).into(), _query_impl!(@index_kind $kind), _query_impl!(@key_type $type))
    );
    // unique
    (@index_fields $args:tt, [ $($index:tt)* ], $($field:ident).+ $type:ident unique $($tokens:tt)*) => (
        _query_impl!(@index_fields $args, [ $($index)* {_query_impl!(@index_field $($field).+, uni, $type)} ], $($tokens)*)
    );
    // duplicate
    (@index_fields $args:tt, [ $($index:tt)* ], $($field:ident).+ $type:ident $($tokens:tt)*) => (
        _query_impl!(@index_fields $args, [ $($index)* {_query_impl!(@index_field $($field).+, dup, $type)} ], $($tokens)*)
    );
    // skip comma
    (@index_fields $args:tt, $index:tt, , $($tokens:tt)*) => (
        _query_impl!(@index_fields $args, $index, $($tokens)*)
    );
    // skip semicolon
    (@index_fields $args:tt, $index:tt, ; $($tokens:tt)*) => (
        _query_impl!(@index_fields $args, $index, $($tokens)*)
    );
    // end fields
    (@index_fields ($out:ident, $coll:expr), [ $($index:tt)+ ], $(,)*) => (
        _query_impl!(@call $out, @index $coll, $($index),+)
    );
    // index kinds
    (@index_kind unique) => ( $crate::IndexKind::Unique );
    (@index_kind uniq) => ( $crate::IndexKind::Unique );
    (@index_kind uni) => ( $crate::IndexKind::Unique );
    (@index_kind duplicate) => ( $crate::IndexKind::Duplicate );
    (@index_kind dupl) => ( $crate::IndexKind::Duplicate );
    (@index_kind dup) => ( $crate::IndexKind::Duplicate );
    // key types
    (@key_type integer) => ( $crate::KeyType::Int );
    (@key_type int) => ( $crate::KeyType::Int );
    (@key_type int64) => ( $crate::KeyType::Int );
    (@key_type float) => ( $crate::KeyType::Float );
    (@key_type float64) => ( $crate::KeyType::Float );
    (@key_type boolean) => ( $crate::KeyType::Bool );
    (@key_type bool) => ( $crate::KeyType::Bool );
    (@key_type string) => ( $crate::KeyType::String );
    (@key_type str) => ( $crate::KeyType::String );
    (@key_type text) => ( $crate::KeyType::String );
    (@key_type binary) => ( $crate::KeyType::Binary );
    (@key_type bin) => ( $crate::KeyType::Binary );
    (@key_type bytes) => ( $crate::KeyType::Binary );

    // find query
    (@find $args:tt,) => (
        _query_impl!(@find_impl $args, [], [])
    );
    (@find $args:tt, order $($order:tt)+) => (
        _query_impl!(@find_impl $args, [], [ $($order)+ ])
    );
    (@find $args:tt, where $($tokens:tt)+) => (
        _query_impl!(@find_filter $args, [], $($tokens)+)
    );
    (@find_filter $args:tt, $filter:tt, order by $($field:ident).+ >) => (
        _query_impl!(@find_impl $args, $filter, [ by $($field).+ > ])
    );
    (@find_filter $args:tt, $filter:tt, order by $($field:ident).+ <) => (
        _query_impl!(@find_impl $args, $filter, [ by $($field).+ < ])
    );
    (@find_filter $args:tt, $filter:tt, order by $($field:ident).+ asc) => (
        _query_impl!(@find_impl $args, $filter, [ by $($field).+ asc ])
    );
    (@find_filter $args:tt, $filter:tt, order by $($field:ident).+ desc) => (
        _query_impl!(@find_impl $args, $filter, [ by $($field).+ desc ])
    );
    (@find_filter $args:tt, $filter:tt, order by $($field:ident).+) => (
        _query_impl!(@find_impl $args, $filter, [ by $($field).+ ])
    );
    (@find_filter $args:tt, $filter:tt, order >) => (
        _query_impl!(@find_impl $args, $filter, [ > ])
    );
    (@find_filter $args:tt, $filter:tt, order <) => (
        _query_impl!(@find_impl $args, $filter, [ < ])
    );
    (@find_filter $args:tt, $filter:tt, order asc) => (
        _query_impl!(@find_impl $args, $filter, [ > ])
    );
    (@find_filter $args:tt, $filter:tt, order desc) => (
        _query_impl!(@find_impl $args, $filter, [ < ])
    );
    (@find_filter $args:tt, $filter:tt, ) => (
        _query_impl!(@find_impl $args, $filter, [])
    );
    (@find_filter $args:tt, [ $($filter:tt)* ], $token:tt $($tokens:tt)*) => (
        _query_impl!(@find_filter $args, [ $($filter)* $token ], $($tokens)*)
    );
    (@find_impl ($out:ident, $type:tt, $coll:expr), [ $($filter:tt)* ], [ $($order:tt)* ]) => (
        _query_impl!(@call $out, @find $type, $coll, _query_impl!(@filter $($filter)*), _query_impl!(@order $($order)*))
    );

    // insert query
    (@insert $args:tt, { $($json:tt)* }) => ( // json
        _query_impl!(@insert_impl $args, _query_impl!(@json { $($json)* }))
    );
    (@insert $args:tt, $($data:tt)+) => ( // json
        _query_impl!(@insert_impl $args, $($data)+)
    );
    (@insert_impl ($out:ident, $coll:expr), $doc:expr) => (
        _query_impl!(@call $out, @insert $coll, $doc)
    );

    // update query
    (@update $args:tt, modify $($tokens:tt)+) => (
        _query_impl!(@update_modify $args, [], $($tokens)+)
    );
    (@update_modify $args:tt, $modify:tt, where ( $($conds:tt)* ) $($tokens:tt)*) => (
        _query_impl!(@update_impl $args, [ ( $($conds)* ) $($tokens)* ], $modify)
    );
    (@update_modify $args:tt, $modify:tt, where $($field:ident).+ $($tokens:tt)+) => (
        _query_impl!(@update_impl $args, [ $($field).+ $($tokens)+ ], $modify)
    );
    (@update_modify $args:tt, $modify:tt, where ! $($tokens:tt)+) => (
        _query_impl!(@update_impl $args, [ ! $($tokens)+ ], $modify)
    );
    (@update_modify $args:tt, $modify:tt, ) => (
        _query_impl!(@update_impl $args, [], $modify)
    );
    (@update_modify $args:tt, [ $($modify:tt)* ], $token:tt $($tokens:tt)*) => (
        _query_impl!(@update_modify $args, [ $($modify)* $token ], $($tokens)*)
    );
    (@update_impl ($out:ident, $coll:expr), [ $($filter:tt)* ], [ $($modify:tt)* ]) => (
        _query_impl!(@call $out, @update $coll, _query_impl!(@filter $($filter)*), _query_impl!(@modify $($modify)*))
    );

    // remove query
    (@remove $args:tt, ) => (
        _query_impl!(@remove_impl $args, [])
    );
    (@remove $args:tt, where $($filter:tt)+) => (
        _query_impl!(@remove_impl $args, [ $($filter)+ ])
    );
    (@remove_impl ($out:ident, $coll:expr), [ $($filter:tt)* ]) => (
        _query_impl!(@call $out, @remove $coll, _query_impl!(@filter $($filter)*))
    );

    //
    // Utils
    //

    //
    // Order util
    //
    (@order_kind >) => ( $crate::OrderKind::Asc );
    (@order_kind <) => ( $crate::OrderKind::Desc );
    (@order_kind asc) => ( $crate::OrderKind::Asc );
    (@order_kind desc) => ( $crate::OrderKind::Desc );
    (@order_kind ) => ( $crate::OrderKind::default() );

    (@order by $($field:ident).+ >) => ( _query_impl!(@order_field_impl $($field).+, >) );
    (@order by $($field:ident).+ <) => ( _query_impl!(@order_field_impl $($field).+, <) );
    (@order by $($field:ident).+ asc) => ( _query_impl!(@order_field_impl $($field).+, asc) );
    (@order by $($field:ident).+ desc) => ( _query_impl!(@order_field_impl $($field).+, desc) );
    (@order by $($field:ident).+ ) => ( _query_impl!(@order_field_impl $($field).+, ) );

    (@order $($order:tt)*) => ( $crate::Order::primary(_query_impl!(@order_kind $($order)*)) );

    (@order_field_impl $($field:ident).+, $($order:tt)*) => (
        $crate::Order::field(_query_impl!(@field $($field).+), _query_impl!(@order_kind $($order)*))
    );

    //
    // Filter util
    //
    (@filter ( $($tokens:tt)+ )) => (
        Some(_query_impl!(@filter_or [] $($tokens)+))
    );

    (@filter $($tokens:tt)+) => (
        Some(_query_impl!(@filter_or [] $($tokens)+))
    );

    (@filter ) => (
        (None as Option<$crate::Filter>)
    );

    // start || condition
    (@filter_or [ $($conds:tt)* ] $token:tt $($tokens:tt)*) => (
        _query_impl!(@filter_or_cond [ $($conds)* ] [ $token ] $($tokens)*)
    );
    // end || condition
    (@filter_or [ $($conds:tt)+ ]) => (
        _query_impl!(@filter_or_proc $($conds)+)
    );

    // end operand of || condition
    (@filter_or_cond [ $($conds:tt)* ] [ $($cond:tt)+ ] || $($tokens:tt)*) => (
        _query_impl!(@filter_or [ $($conds)* [ $($cond)+ ] ] $($tokens)*)
    );
    // end operand of || condition
    (@filter_or_cond [ $($conds:tt)* ] [ $($cond:tt)+ ]) => (
        _query_impl!(@filter_or [ $($conds)* [ $($cond)+ ] ])
    );
    // add token to current operand of || condition
    (@filter_or_cond [ $($conds:tt)* ] [ $($cond:tt)+ ] $token:tt $($tokens:tt)*) => (
        _query_impl!(@filter_or_cond [ $($conds)* ] [ $($cond)+ $token ] $($tokens)*)
    );

    // process single || condition
    (@filter_or_proc [ $($tokens:tt)+ ]) => (
        _query_impl!(@filter_and [] $($tokens)+)
    );
    // process multiple || conditions
    (@filter_or_proc $($cond:tt)+) => (
        $crate::Filter::Cond($crate::Cond::Or(_query_impl!(@vec $(_query_impl!(@filter_and_wrap $cond)),+)))
    );

    (@filter_and_wrap [ $($tokens:tt)+ ]) => (
        _query_impl!(@filter_and [] $($tokens)+)
    );

    // start && condition
    (@filter_and [ $($conds:tt)* ] $token:tt $($tokens:tt)*) => (
        _query_impl!(@filter_and_cond [ $($conds)* ] [ $token ] $($tokens)*)
    );
    // end && condition
    (@filter_and [ $($conds:tt)+ ]) => (
        _query_impl!(@filter_and_proc $($conds)+)
    );

    // end operand of && condition
    (@filter_and_cond [ $($conds:tt)* ] [ $($cond:tt)+ ] && $($tokens:tt)*) => (
        _query_impl!(@filter_and [ $($conds)* [ $($cond)+ ] ] $($tokens)*)
    );
    // end operand of && condition
    (@filter_and_cond [ $($conds:tt)* ] [ $($cond:tt)+ ]) => (
        _query_impl!(@filter_and [ $($conds)* [ $($cond)+ ] ])
    );
    // add token to current operand of && condition
    (@filter_and_cond [ $($conds:tt)* ] [ $($cond:tt)+ ] $token:tt $($tokens:tt)*) => (
        _query_impl!(@filter_and_cond [ $($conds)* ] [ $($cond)+ $token ] $($tokens)*)
    );

    // process single && condition
    (@filter_and_proc [ $($tokens:tt)+ ]) => (
        _query_impl!(@filter_not $($tokens)+)
    );
    // process multiple && conditions
    (@filter_and_proc $($cond:tt)+) => (
        $crate::Filter::Cond($crate::Cond::And(_query_impl!(@vec $(_query_impl!(@filter_not_wrap $cond)),+)))
    );

    (@filter_not_wrap [ $($tokens:tt)+ ]) => (
        _query_impl!(@filter_not $($tokens)+)
    );

    // parse !
    (@filter_not ! $($tokens:tt)+) => (
        $crate::Filter::Cond($crate::Cond::Not(Box::new(_query_impl!(@filter_nest $($tokens)+))))
    );

    // parse !!
    (@filter_not $($tokens:tt)+) => (
        _query_impl!(@filter_nest $($tokens)+)
    );

    // parse ()-enclosed sub-expressions
    (@filter_nest ( $($tokens:tt)+ )) => (
        _query_impl!(@filter_or [] $($tokens)+)
    );

    // parse expression
    (@filter_nest $($tokens:tt)+) => (
        _query_impl!(@filter_comp $($tokens)+)
    );

    // equal
    (@filter_comp $($field:ident).+ == $value:expr) => (
        _query_impl!(@filter_comp_impl $($field).+, Eq, $crate::KeyData::from($value))
    );
    // not equal
    (@filter_comp $($field:ident).+ != $value:expr) => (
        _query_impl!(@filter_comp_impl ! $($field).+, Eq, $crate::KeyData::from($value))
    );
    // out of set (not one of)
    (@filter_comp $($field:ident).+ !of [$($value:expr),*]) => (
        _query_impl!(@filter_comp_impl ! $($field).+, In, _query_impl!(@vec $($crate::KeyData::from($value)),*))
    );
    // in set (one of)
    (@filter_comp $($field:ident).+ of [$($value:expr),*]) => (
        _query_impl!(@filter_comp_impl $($field).+, In, _query_impl!(@vec $($crate::KeyData::from($value)),*))
    );
    // out of set (not one of)
    (@filter_comp $($field:ident).+ !of $value:expr) => (
        _query_impl!(@filter_comp_impl ! $($field).+, In, $value.into_iter().map($crate::KeyData::from).collect())
    );
    // in set (one of)
    (@filter_comp $($field:ident).+ of $value:expr) => (
        _query_impl!(@filter_comp_impl $($field).+, In, $value.into_iter().map($crate::KeyData::from).collect())
    );

    // less than
    (@filter_comp $($field:ident).+ < $value:expr) => (
        _query_impl!(@filter_comp_impl $($field).+, Lt, $crate::KeyData::from($value))
    );
    // less than or equal
    (@filter_comp $($field:ident).+ <= $value:expr) => (
        _query_impl!(@filter_comp_impl $($field).+, Le, $crate::KeyData::from($value))
    );

    // greater than
    (@filter_comp $($field:ident).+ > $value:expr) => (
        _query_impl!(@filter_comp_impl $($field).+, Gt, $crate::KeyData::from($value))
    );
    // greater than or equal
    (@filter_comp $($field:ident).+ >= $value:expr) => (
        _query_impl!(@filter_comp_impl $($field).+, Ge, $crate::KeyData::from($value))
    );

    // in bounded range
    (@filter_comp $($field:ident).+ in $range:expr) => (
        _query_impl!(@filter_comp_impl $($field).+, Bw, $crate::KeyData::from($range.start), true, $crate::KeyData::from($range.end), true)
    );

    // in bounded range excluding bounds
    (@filter_comp $($field:ident).+ <in> $range:expr) => (
        _query_impl!(@filter_comp_impl $($field).+, Bw, $crate::KeyData::from($range.start), false, $crate::KeyData::from($range.end), false)
    );

    // in bounded range excluding start (left) bound
    (@filter_comp $($field:ident).+ <in $range:expr) => (
        _query_impl!(@filter_comp_impl $($field).+, Bw, $crate::KeyData::from($range.start), false, $crate::KeyData::from($range.end), true)
    );

    // in bounded range excluding end (right) bound
    (@filter_comp $($field:ident).+ in> $range:expr) => (
        _query_impl!(@filter_comp_impl $($field).+, Bw, $crate::KeyData::from($range.start), true, $crate::KeyData::from($range.end), false)
    );

    // has value (field exists or not null)
    (@filter_comp $($field:ident).+ ?) => (
        _query_impl!(@filter_comp_impl $($field).+, Has)
    );

    (@filter_comp_impl ! $($tokens:tt)+) => (
        $crate::Filter::cond($crate::Cond::Not(Box::new(_query_impl!(@filter_comp_impl $($tokens)+))))
    );

    (@filter_comp_impl $($field:ident).+, $op:ident) => (
        $crate::Filter::comp(_query_impl!(@field $($field).+), $crate::Comp::$op)
    );

    (@filter_comp_impl $($field:ident).+, $op:ident, $($args:expr),+) => (
        $crate::Filter::comp(_query_impl!(@field $($field).+), $crate::Comp::$op($($args),+))
    );

    //
    // Modify util
    //
    (@modify $($tokens:tt)+) => ( _query_impl!(@modify_parse [] $($tokens)*) );

    // none modifications
    (@modify ) => ( $crate::Modify::default() );

    // parsing
    (@modify_parse [ $($actions:tt)* ] , $($tokens:tt)*) => ( // skip commas at top level
        _query_impl!(@modify_parse [ $($actions)* ] $($tokens)*)
    );
    (@modify_parse [ $($actions:tt)* ] ; $($tokens:tt)*) => ( // skip semicolons at top level
        _query_impl!(@modify_parse [ $($actions)* ] $($tokens)*)
    );
    (@modify_parse [ $($actions:tt)* ] ) => ( // goto processing
        _query_impl!(@modify_apply $($actions)*)
    );
    (@modify_parse [ $($actions:tt)* ] $token:tt $($tokens:tt)*) => ( // start action parsing
        _query_impl!(@modify_action_parse [ $($actions)* ] { $token } $($tokens)*)
    );
    // parsing action
    (@modify_action_parse [ $($actions:tt)* ] { $($action:tt)+ } , $($tokens:tt)*) => ( // end action parsing
        _query_impl!(@modify_parse [ $($actions)* { $($action)+ } ] $($tokens)*)
    );
    (@modify_action_parse [ $($actions:tt)* ] { $($action:tt)+ } ; $($tokens:tt)*) => ( // end action parsing
        _query_impl!(@modify_parse [ $($actions)* { $($action)+ } ] $($tokens)*)
    );
    (@modify_action_parse [ $($actions:tt)* ] { $($action:tt)+ } ) => ( // end action parsing
        _query_impl!(@modify_parse [ $($actions)* { $($action)+ } ])
    );
    (@modify_action_parse [ $($actions:tt)* ] { $($action:tt)+ } $token:tt $($tokens:tt)*) => ( // action parsing
        _query_impl!(@modify_action_parse [ $($actions)* ] { $($action)+ $token } $($tokens)*)
    );
    // processing
    (@modify_apply $($actions:tt)*) => ({ // top level processing
        let mut m = $crate::Modify::default();
        _query_impl!(@modify_actions_apply m $($actions)*);
        m
    });
    (@modify_actions_apply $m:ident { $($action:tt)+ } $($actions:tt)*) => (
        _query_impl!(@modify_action_apply $m $($action)+);
        _query_impl!(@modify_actions_apply $m $($actions)*)
    );
    (@modify_actions_apply $m:ident ) => (
    );
    // processing action
    // field = value
    (@modify_action_apply $m:ident $($field:ident).+ = $val:expr) => (
        $m.add(_query_impl!(@field $($field).+), $crate::Action::Set($crate::to_value($val).unwrap()))
    );
    // field ~ (delete)
    (@modify_action_apply $m:ident $($field:ident).+ ~) => (
        $m.add(_query_impl!(@field $($field).+), $crate::Action::Delete)
    );
    // field += value
    (@modify_action_apply $m:ident $($field:ident).+ += $val:expr) => (
        $m.add(_query_impl!(@field $($field).+), $crate::Action::Add($crate::to_value($val).unwrap()))
    );
    // field -= value
    (@modify_action_apply $m:ident $($field:ident).+ -= $val:expr) => (
        $m.add(_query_impl!(@field $($field).+), $crate::Action::Sub($crate::to_value($val).unwrap()))
    );
    // field *= value
    (@modify_action_apply $m:ident $($field:ident).+ *= $val:expr) => (
        $m.add(_query_impl!(@field $($field).+), $crate::Action::Mul($crate::to_value($val).unwrap()))
    );
    // field /= value
    (@modify_action_apply $m:ident $($field:ident).+ /= $val:expr) => (
        $m.add(_query_impl!(@field $($field).+), $crate::Action::Div($crate::to_value($val).unwrap()))
    );
    // field ! (toggle)
    (@modify_action_apply $m:ident $($field:ident).+ !) => (
        $m.add(_query_impl!(@field $($field).+), $crate::Action::Toggle)
    );
    // splice helper
    (@modify_add_splice $m:ident $($field:ident).+ [ - $start:tt .. - $delete:tt ] $($insert:tt)*) => (
        $m.add(_query_impl!(@field $($field).+), $crate::Action::Splice(-$start, -$delete, _query_impl!(@modify_insert_splice $($insert)*)))
    );
    (@modify_add_splice $m:ident $($field:ident).+ [ - $start:tt .. $delete:tt ] $($insert:tt)*) => (
        $m.add(_query_impl!(@field $($field).+), $crate::Action::Splice(-$start, $delete, _query_impl!(@modify_insert_splice $($insert)*)))
    );
    (@modify_add_splice $m:ident $($field:ident).+ [ $start:tt .. - $delete:tt ] $($insert:tt)*) => (
        $m.add(_query_impl!(@field $($field).+), $crate::Action::Splice($start, -$delete, _query_impl!(@modify_insert_splice $($insert)*)))
    );
    (@modify_add_splice $m:ident $($field:ident).+ [ $start:tt .. $delete:tt ] $($insert:tt)*) => (
        $m.add(_query_impl!(@field $($field).+), $crate::Action::Splice($start, $delete, _query_impl!(@modify_insert_splice $($insert)*)))
    );
    (@modify_add_splice $m:ident $($field:ident).+ [ - $start:tt .. ] $($insert:tt)*) => (
        $m.add(_query_impl!(@field $($field).+), $crate::Action::Splice(-$start, -1, _query_impl!(@modify_insert_splice $($insert)*)))
    );
    (@modify_add_splice $m:ident $($field:ident).+ [ $start:tt .. ] $($insert:tt)*) => (
        $m.add(_query_impl!(@field $($field).+), $crate::Action::Splice($start, -1, _query_impl!(@modify_insert_splice $($insert)*)))
    );
    (@modify_add_splice $m:ident $($field:ident).+ [ .. - $delete:tt ] $($insert:tt)*) => (
        $m.add(_query_impl!(@field $($field).+), $crate::Action::Splice(0, -$delete, _query_impl!(@modify_insert_splice $($insert)*)))
    );
    (@modify_add_splice $m:ident $($field:ident).+ [ .. $delete:tt ] $($insert:tt)*) => (
        $m.add(_query_impl!(@field $($field).+), $crate::Action::Splice(0, $delete, _query_impl!(@modify_insert_splice $($insert)*)))
    );
    (@modify_add_splice $m:ident $($field:ident).+ [ .. ] $($insert:tt)*) => (
        $m.add(_query_impl!(@field $($field).+), $crate::Action::Splice(0, -1, _query_impl!(@modify_insert_splice $($insert)*)))
    );
    (@modify_insert_splice $insert:expr) => (
        $insert.iter().map(|elm| $crate::to_value(elm).unwrap()).collect()
    );
    (@modify_insert_splice ) => (
        Vec::new()
    );
    // remove from an array
    (@modify_action_apply $m:ident $($field:ident).+ [ $($range:tt)+ ] ~) => (
        _query_impl!(@modify_add_splice $m $($field).+ [ $($range)+ ])
    );
    // splice array
    (@modify_action_apply $m:ident $($field:ident).+ [ $($range:tt)+ ] = $insert:expr) => (
        _query_impl!(@modify_add_splice $m $($field).+ [ $($range)+ ] $insert)
    );
    // merge object
    (@modify_action_apply $m:ident $($field:ident).+ ~= { $($obj:tt)+ }) => (
        $m.add(_query_impl!(@field $($field).+), $crate::Action::Merge($crate::to_value(_query_impl!(@json { $($obj)+ })).unwrap()))
    );
    (@modify_action_apply $m:ident $($field:ident).+ ~= $obj:expr) => (
        $m.add(_query_impl!(@field $($field).+), $crate::Action::Merge($crate::to_value($obj).unwrap()))
    );
    // string replace
    (@modify_action_apply $m:ident $($field:ident).+ ~= $pat:tt $sub:expr) => (
        $m.add(_query_impl!(@field $($field).+), $crate::Action::Replace($crate::WrappedRegex($pat.parse().unwrap()), String::from($sub)))
    );
    //(@modify_action_apply $m:ident $($any:tt)+) => ( println!("!! {:?}", _query_impl!(@tts $($any)+)) );

    //
    // Basic utils
    //

    // field name (as str)
    (@field $($part:tt)+) => (
        _query_impl!(@concat $(_query_impl!(@stringify $part)),+)
    );

    // debug util (for dev only)
    (@tts $($x:tt)+) => (
        _query_impl!(@vec $(_query_impl!(@stringify $x)),+)
    );

    (@call $cb:ident, $($args:tt)*) => ( _query_extr!(@call $cb, $($args)*) );

    (@vec $($content:tt)*) => ( _query_extr!(@vec $($content)*) );
    
    (@stringify $($content:tt)*) => ( _query_extr!(@stringify $($content)*) );
    
    (@concat $($content:tt)*) => ( _query_extr!(@concat $($content)*) );

    (@json $($content:tt)*) => ( _query_extr!(@json $($content)*) );
}

#[macro_export]
#[doc(hidden)]
macro_rules! _query_extr {
    (@call $cb:ident, $($args:tt)*) => ( $cb!($($args)*) );
    (@vec $($content:tt)*) => ( vec! [ $($content)* ] );
    (@stringify $($content:tt)*) => ( stringify! { $($content)* } );
    (@concat $($content:tt)*) => ( concat! { $($content)* } );
    (@json $($content:tt)*) => ( json! { $($content)* } );
}

#[cfg(test)]
mod test {
    mod filter {
        use serde_json::from_value;

        #[test]
        fn empty() {
            assert_eq!(query!(@filter ), json_val!(null));
        }

        #[test]
        fn comp_eq() {
            assert_eq!(query!(@filter f == 123), json_val!({ "f": { "$eq": 123 } }));
            assert_eq!(
                query!(@filter f != 123),
                json_val!({ "$not": { "f": { "$eq": 123 } } })
            );
            assert_eq!(
                query!(@filter f != "abc"),
                json_val!({ "$not": { "f": { "$eq": "abc" } } })
            );
        }

        #[test]
        fn comp_in() {
            assert_eq!(
                query!(@filter f of [1, 2, 3]),
                json_val!({ "f": { "$in": [1, 2, 3] } })
            );
            assert_eq!(
                query!(@filter f !of [1, 2, 3]),
                json_val!({ "$not": { "f": { "$in": [1, 2, 3] } } })
            );
            assert_eq!(
                query!(@filter f of ["a", "b", "c"]),
                json_val!({ "f": { "$in": ["a", "b", "c"] } })
            );
            // variants
            let v = [1, 2, 3];
            assert_eq!(
                query!(@filter f of v),
                json_val!({ "f": { "$in": v } })
            );
        }

        #[test]
        fn comp_lt() {
            assert_eq!(query!(@filter f < 123), json_val!({ "f": { "$lt": 123 } }));
        }

        #[test]
        fn comp_le() {
            assert_eq!(query!(@filter f <= 123), json_val!({ "f": { "$le": 123 } }));
        }

        #[test]
        fn comp_gt() {
            assert_eq!(query!(@filter f > 123), json_val!({ "f": { "$gt": 123 } }));
        }

        #[test]
        fn comp_ge() {
            assert_eq!(query!(@filter f >= 123), json_val!({ "f": { "$ge": 123 } }));
        }

        #[test]
        fn comp_bw() {
            assert_eq!(
                query!(@filter f in 12..34),
                json_val!({ "f": { "$bw": [12, true, 34, true] } })
            );
            assert_eq!(
                query!(@filter f <in> 12..34),
                json_val!({ "f": { "$bw": [12, false, 34, false] } })
            );
            assert_eq!(
                query!(@filter f <in 12..34),
                json_val!({ "f": { "$bw": [12, false, 34, true] } })
            );
            assert_eq!(
                query!(@filter f in> 12..34),
                json_val!({ "f": { "$bw": [12, true, 34, false] } })
            );

            assert_eq!(
                query!(@filter f in -12..-34),
                json_val!({ "f": { "$bw": [-12, true, -34, true] } })
            );
            assert_eq!(
                query!(@filter f <in> -12..-34),
                json_val!({ "f": { "$bw": [-12, false, -34, false] } })
            );
            assert_eq!(
                query!(@filter f <in -12..-34),
                json_val!({ "f": { "$bw": [-12, false, -34, true] } })
            );
            assert_eq!(
                query!(@filter f in> -12..-34),
                json_val!({ "f": { "$bw": [-12, true, -34, false] } })
            );
        }

        #[test]
        fn comp_has() {
            assert_eq!(query!(@filter f?), json_val!({ "f": "$has" }));
        }

        #[test]
        fn cond_not() {
            assert_eq!(query!(@filter !a?), json_val!({ "$not": { "a": "$has" } }));
        }

        #[test]
        fn cond_and() {
            assert_eq!(
                query!(@filter a == 3 && b >= 1),
                json_val!({ "$and": [ { "a": { "$eq": 3 } }, { "b": { "$ge": 1 } } ] })
            );
        }

        #[test]
        fn cond_or() {
            assert_eq!(
                query!(@filter a == "abc" || b <in> 12..34),
                json_val!({ "$or": [ { "a": { "$eq": "abc" } }, { "b": { "$bw": [12, false, 34, false] } } ] })
            );
        }

        #[test]
        fn cond_not_and() {
            assert_eq!(
                query!(@filter !(a == "abc" && b <in> 12..34)),
                json_val!({ "$not": { "$and": [ { "a": { "$eq": "abc" } }, { "b": { "$bw": [12, false, 34, false] } } ] } })
            );
        }

        #[test]
        fn cond_and_not() {
            assert_eq!(
                query!(@filter a != "abc" && !(b <in> 12..34)),
                json_val!({ "$and": [ { "$not": { "a": { "$eq": "abc" } } }, { "$not": { "b": { "$bw": [12, false, 34, false] } } } ] })
            );
        }

        #[test]
        fn cond_and_or() {
            let b_edge = 10;

            assert_eq!(
                query!(@filter a of [1, 2, 3] && (b > b_edge || b < -b_edge)),
                json_val!({ "$and": [ { "a": { "$in": [1, 2, 3] } }, { "$or": [ { "b": { "$gt": 10 } }, { "b": { "$lt": -10 } } ] } ] })
            );
        }

        #[test]
        fn comp_sub_fields() {
            assert_eq!(
                query!(@filter a.b.c == 1),
                json_val!({ "a.b.c": { "$eq": 1 } })
            );
        }

        #[test]
        fn or_nested_and() {
            assert_eq!(
                query!(@filter a == 1 || !b == "abc" && c < 5),
                json_val!({ "$or": [
                           { "a": { "$eq": 1 } },
                           { "$and": [
                               { "$not": { "b": { "$eq": "abc" } } },
                               { "c": { "$lt": 5 } }
                           ] }
                       ] })
            );
            assert_eq!(
                query!(@filter a == 1 && !b == "abc" || c < 5),
                json_val!({ "$or": [
                           { "$and": [
                               { "a": { "$eq": 1 } },
                               { "$not": { "b": { "$eq": "abc" } } }
                           ] },
                           { "c": { "$lt": 5 } }
                       ] })
            );
        }

        #[test]
        fn and_nested_or() {
            assert_eq!(
                query!(@filter (a == 1 || !b == "abc") && c < 5),
                json_val!({ "$and": [
                           { "$or": [
                               { "a": { "$eq": 1 } },
                               { "$not": { "b": { "$eq": "abc" } } }
                           ] },
                           { "c": { "$lt": 5 } }
                       ] })
            );
            assert_eq!(
                query!(@filter a == 1 && (!b == "abc" || c < 5)),
                json_val!({ "$and": [
                           { "a": { "$eq": 1 } },
                           { "$or": [
                               { "$not": { "b": { "$eq": "abc" } } },
                               { "c": { "$lt": 5 } }
                           ] }
                       ] })
            );
        }
    }

    mod order {
        use serde_json::from_value;

        #[test]
        fn default() {
            assert_eq!(query!(@order ), json_val!("$asc"));
        }

        #[test]
        fn primary_asc() {
            assert_eq!(query!(@order >), json_val!("$asc"));
            assert_eq!(query!(@order asc), json_val!("$asc"));
        }

        #[test]
        fn primary_desc() {
            assert_eq!(query!(@order <), json_val!("$desc"));
            assert_eq!(query!(@order desc), json_val!("$desc"));
        }

        #[test]
        fn field_asc() {
            assert_eq!(query!(@order by field >), json_val!({ "field": "$asc" }));
            assert_eq!(query!(@order by field asc), json_val!({ "field": "$asc" }));
        }

        #[test]
        fn field_desc() {
            assert_eq!(query!(@order by a.b.c <), json_val!({ "a.b.c": "$desc" }));
            assert_eq!(
                query!(@order by a.b.c desc),
                json_val!({ "a.b.c": "$desc" })
            );
        }
    }

    mod modify {
        use serde_json::from_value;

        #[test]
        fn empty() {
            assert_eq!(query!(@modify ), json_val!({}));
        }

        #[test]
        fn set() {
            assert_eq!(query!(@modify a = 1u32), json_val!({ "a": { "$set": 1 } }));
            assert_eq!(
                query!(@modify a = 123u32, b.c = "abc"),
                json_val!({ "a": { "$set": 123 }, "b.c": { "$set": "abc" } })
            );
            assert_eq!(
                query!(@modify 
                a = 123u32;
                b.c = "abc";
            ),
                json_val!({ "a": { "$set": 123 }, "b.c": { "$set": "abc" } })
            );
        }

        #[test]
        fn delete() {
            assert_eq!(query!(@modify field ~), json_val!({ "field": "$delete" }));
            assert_eq!(
                query!(@modify field ~, other.field ~),
                json_val!({ "field": "$delete", "other.field": "$delete" })
            );
        }

        #[test]
        fn add() {
            assert_eq!(
                query!(@modify field += 123u32),
                json_val!({ "field": { "$add": 123 } })
            );
            assert_eq!(
                query!(@modify field += 123u32, other.field += "abc"),
                json_val!({ "field": { "$add": 123 }, "other.field": { "$add": "abc" } })
            );
        }

        #[test]
        fn sub() {
            assert_eq!(
                query!(@modify field -= 123u32),
                json_val!({ "field": { "$sub": 123 } })
            );
            assert_eq!(
                query!(@modify field -= 123u32, other.field -= "abc"),
                json_val!({ "field": { "$sub": 123 }, "other.field": { "$sub": "abc" } })
            );
        }

        #[test]
        fn mul() {
            assert_eq!(
                query!(@modify field *= 123u32),
                json_val!({ "field": { "$mul": 123 } })
            );
            assert_eq!(
                query!(@modify field *= 123u32, other.field *= "abc"),
                json_val!({ "field": { "$mul": 123 }, "other.field": { "$mul": "abc" } })
            );
        }

        #[test]
        fn div() {
            assert_eq!(
                query!(@modify field /= 123u32),
                json_val!({ "field": { "$div": 123 } })
            );
            assert_eq!(
                query!(@modify field /= 123u32, other.field /= "abc"),
                json_val!({ "field": { "$div": 123 }, "other.field": { "$div": "abc" } })
            );
        }

        #[test]
        fn toggle() {
            assert_eq!(query!(@modify field!), json_val!({ "field": "$toggle" }));
            assert_eq!(
                query!(@modify field!, other.field!),
                json_val!({ "field": "$toggle", "other.field": "$toggle" })
            );
            assert_eq!(
                query!(@modify field!; field!),
                json_val!({ "field": ["$toggle", "$toggle"] })
            );
        }

        #[test]
        fn replace() {
            assert_eq!(
                query!(@modify field ~= "abc" "def"),
                json_val!({ "field": { "$replace": ["abc", "def"] } })
            );
            assert_eq!(
                query!(@modify field ~= "abc" "def", other.field ~= "april" "may"),
                json_val!({ "field": { "$replace": ["abc", "def"] }, "other.field": { "$replace": ["april", "may"] } })
            );
        }

        #[test]
        fn splice() {
            assert_eq!(
                query!(@modify field[1..2]~),
                json_val!({ "field": { "$splice": [1, 2] } })
            );

            assert_eq!(
                query!(@modify field[1..2] = ["a", "b", "c"]),
                json_val!({ "field": { "$splice": [1, 2, "a", "b", "c"] } })
            );

            let ins = [1u8, 2, 3];
            assert_eq!(
                query!(@modify other.field[-1..0] = ins),
                json_val!({ "other.field": { "$splice": [-1, 0, 1, 2, 3] } })
            );

            assert_eq!(
                query!(@modify field[..]~),
                json_val!({ "field": { "$splice": [0, -1] } })
            );
        }

        #[test]
        fn merge() {
            #[derive(Serialize, Deserialize)]
            struct Extra {
                subfield: bool,
                other: u8,
            }

            let extra = Extra {
                subfield: true,
                other: 123,
            };
            assert_eq!(
                query!(@modify field ~= extra),
                json_val!({ "field": { "$merge": { "subfield": true, "other": 123 } } })
            );

            assert_eq!(
                query!(@modify field ~= { "subfield": true, "other": 123 }),
                json_val!({ "field": { "$merge": { "subfield": true, "other": 123 } } })
            );
        }
    }
}
