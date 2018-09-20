use lmdb::Database;
use std::mem::replace;
use std::ops::Deref;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use supercow::{ext::ConstDeref, Supercow};

#[derive(Clone)]
pub(crate) struct WrappedDatabase {
    // Database handle
    db: Option<Arc<Database<'static>>>,
    // Remove marker
    rm: Arc<AtomicBool>,
}

impl WrappedDatabase {
    pub fn new(db: Database<'static>) -> Self {
        WrappedDatabase {
            db: Some(Arc::new(db)),
            rm: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn delete_on_drop(&self, on: bool) {
        self.rm.store(on, Ordering::SeqCst);
    }
}

impl Drop for WrappedDatabase {
    fn drop(&mut self) {
        let db = replace(&mut self.db, None).unwrap();
        if self.rm.load(Ordering::SeqCst) {
            if let Ok(db) = Arc::try_unwrap(db) {
                if let Err(e) = db.delete() {
                    eprintln!("Error when deleting db: {}", e);
                }
            }
        }
    }
}

impl Deref for WrappedDatabase {
    type Target = Database<'static>;

    #[inline]
    fn deref(&self) -> &Self::Target {
        if let Some(db_ref) = &self.db {
            db_ref.deref()
        } else {
            unreachable!()
        }
    }
}

unsafe impl ConstDeref for WrappedDatabase {
    type Target = Database<'static>;

    #[inline]
    fn const_deref(&self) -> &Self::Target {
        if let Some(db_ref) = &self.db {
            db_ref.const_deref()
        } else {
            unreachable!()
        }
    }
}

impl<'a> Into<Supercow<'a, Database<'a>>> for WrappedDatabase {
    fn into(self) -> Supercow<'a, Database<'a>> {
        let this = self.clone();
        Supercow::shared(this)
    }
}

#[cfg(test)]
mod test {
    use super::WrappedDatabase;
    use lmdb::{open, Database, DatabaseOptions, EnvBuilder, ReadTransaction};
    use std::sync::Arc;

    #[test]
    #[ignore]
    fn test() {
        let env = Arc::new(unsafe {
            EnvBuilder::new()
                .unwrap()
                .open("test", open::Flags::empty(), 0o600)
                .unwrap()
        });

        let db = WrappedDatabase::new(
            Database::open(env.clone(), None, &DatabaseOptions::defaults()).unwrap(),
        );

        let txn = ReadTransaction::new(env.clone()).unwrap();
        let access = txn.access();

        let _val: &str = access.get(&db, "key").unwrap();

        let mut _cursor = txn.cursor(db.clone()).unwrap();
    }
}
