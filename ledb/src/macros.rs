#[doc(hidden)]
pub use ledb_macros::*;

proc_macro_expr_decl! {
    _query_dsl! => _query_dsl_extract_impl
}

#[macro_export]
macro_rules! query {
    (index $($t:tt)+) => {
        _query_dsl!(COLLECTION, index $($t)+).set_indexes(&_query_dsl!(FIELDS, index $($t)+))
    };
    (find in $($t:tt)*) => {
        _query_dsl!(COLLECTION, find in $($t)+).find(_query_dsl!(FILTER, find in $($t)+), _query_dsl!(ORDER, find in $($t)+))
    };
    (find $type:tt in $($t:tt)*) => {
        _query_dsl!(COLLECTION, find $type in $($t)+).find::<$type>(_query_dsl!(FILTER, find $type in $($t)+), _query_dsl!(ORDER, find $type in $($t)+))
    };
    (insert $($t:tt)*) => {
        _query_dsl!(COLLECTION, insert $($t)+).insert(_query_dsl!(DOCUMENT, insert $($t)+))
    };
    (update, $($t:tt)*) => {
        _query_dsl!(COLLECTION, update $($t)+).update(_query_dsl!(FILTER, update $($t)+), _query_dsl!(MODIFY, update $($t)+))
    };
    (remove, $($t:tt)*) => {
        _query_dsl!(COLLECTION, remove $($t)+).remove(_query_dsl!(FILTER, remove $($t)+))
    };
}
