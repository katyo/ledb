use lmdb::{ConstAccessor, Cursor, Result, Error};
use lmdb::traits::{AsLmdbBytes, FromLmdbBytes};
use lmdbffi::MDB_NOTFOUND;

pub trait CursorExtra {
    #[inline]
    fn seek_range_k_prev<'access, K: AsLmdbBytes + FromLmdbBytes + PartialEq + ?Sized, V: FromLmdbBytes + ?Sized>(
        &mut self, 
        access: &'access ConstAccessor, 
        key: &K
    ) -> Result<(&'access K, &'access V)>;
}

impl<'txn,'db> CursorExtra for Cursor<'txn,'db> {
    #[inline]
    fn seek_range_k_prev<'access, K: AsLmdbBytes + FromLmdbBytes + PartialEq + ?Sized, V: FromLmdbBytes + ?Sized>(
        &mut self,
        access: &'access ConstAccessor, 
        key: &K
    ) -> Result<(&'access K, &'access V)> {
        match self.seek_range_k(access, key) {
            Ok((ckey, cval)) => if key != ckey {
                self.prev_nodup(access)
            } else {
                Ok((ckey, cval))
            },
            Err(Error::Code(MDB_NOTFOUND)) => self.last(access),
            Err(e) => Err(e),
        }
    }
}
