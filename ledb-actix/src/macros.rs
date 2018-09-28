/// Unified query message macro
///
#[macro_export]
macro_rules! query {
    // call util
    (@$util:ident $($args:tt)*) => ( _query_impl!(@$util $($args)*) );

    // make query
    ($($tokens:tt)+) => ( _query_impl!(@query _query_actix, $($tokens)+) );
}

// native API output macros
#[macro_export]
#[doc(hidden)]
macro_rules! _query_actix {
    (@index $coll:expr, $($indexes:tt),+) => (
        $crate::SetIndexes(_query_impl!(@stringify $coll), _query_impl![@vec $($indexes),+])
    );
    (@find $type:tt, $coll:expr, $filter:expr, $order:expr) => (
        $crate::Find::<_, $type>(_query_impl!(@stringify $coll), $filter, $order)
    );
    (@insert $coll:expr, $doc:expr) => (
        $crate::Insert(_query_impl!(@stringify $coll), $doc)
    );
    (@update $coll:expr, $filter:expr, $modify:expr) => (
        $crate::Update(_query_impl!(@stringify $coll), $filter, $modify)
    );
    (@remove $coll:expr, $filter:expr) => (
        $crate::Remove(_query_impl!(@stringify $coll), $filter)
    );
}

#[cfg(test)]
mod test {
    use actor::*;
    use ledb::Value;

    #[test]
    fn find() {
        let find_query: FindMsg<Value> = query!(find in collection);
        assert_eq!(find_query, Find("collection", None, query!(@order)));

        assert_eq!(
            query!(find Value in collection),
            Find("collection", None, query!(@order))
        );

        assert_eq!(
            query!(find Value in collection order desc),
            Find("collection", None, query!(@order desc))
        );

        assert_eq!(
            query!(find Value in collection order by field),
            Find("collection", query!(@filter), query!(@order by field asc))
        );

        assert_eq!(
            query!(find Value in collection order by field desc),
            Find("collection", query!(@filter), query!(@order by field desc))
        );

        assert_eq!(
            query!(find Value in collection where field == "abc"),
            Find("collection", query!(@filter field == "abc"), query!(@order))
        );

        assert_eq!(
            query!(find Value in collection where field == "abc" order desc),
            Find(
                "collection",
                query!(@filter field == "abc"),
                query!(@order desc)
            )
        );

        assert_eq!(
            query!(find Value in collection where some.field == "abc" order by other.field desc),
            Find(
                "collection",
                query!(@filter some.field == "abc"),
                query!(@order by other.field <)
            )
        );
    }

    #[test]
    fn update() {
        assert_eq!(
            query!(update in collection modify field = 123),
            Update("collection", query!(@filter), query!(@modify field = 123))
        );

        assert_eq!(
            query!(update in collection modify field.with.sub.field = 123),
            Update(
                "collection",
                query!(@filter),
                query!(@modify field.with.sub.field = 123)
            )
        );

        assert_eq!(
            query!(update in collection modify field.with.sub.field = 123, other.field = "abc"),
            Update(
                "collection",
                query!(@filter),
                query!(@modify field.with.sub.field = 123, other.field = "abc")
            )
        );

        assert_eq!(
            query!(update in collection modify field = "def" where field == "abc"),
            Update(
                "collection",
                query!(@filter field == "abc"),
                query!(@modify field = "def")
            )
        );

        assert_eq!(
            query!(update in collection modify field = "def", other.field += 123, some.flag~ where field == "abc" && some.flag?),
            Update(
                "collection",
                query!(@filter field == "abc" && some.flag?),
                query!(@modify field = "def", other.field += 123, some.flag~)
            )
        );
    }
}
