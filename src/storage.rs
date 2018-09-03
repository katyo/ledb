use std::fs::create_dir_all;
use std::sync::{Arc, RwLock};
use lmdb::{EnvBuilder, Environment, open::Flags as OpenFlags, Database, DatabaseOptions};

pub use types::{Id, Document, Binary, ResultWrap, NOT_FOUND};
pub use key::{IntoKey, FromKey};
pub use collection::{Collection};
pub use index::{Index, IndexKind};

pub struct Storage {
    env: Arc<Environment>,
    collections: RwLock<Vec<Arc<Collection>>>,
}

impl Storage {
    pub fn open<P: AsRef<str>>(path: P) -> Result<Self, String> {
        let path = path.as_ref();
        let mut bld = EnvBuilder::new().wrap_err()?;
        bld.set_maxdbs(1023).wrap_err()?;

        create_dir_all(path).wrap_err()?;
        let env = Arc::new(unsafe { bld.open(path, OpenFlags::empty(), 0o600) }
        .wrap_err()?);

        let db = Arc::new(Database::open(
            env.clone(), None, &DatabaseOptions::defaults())
            .wrap_err()?);

        let collections = RwLock::new(Collection::bootstrap(env.clone(), db.clone())?);
        
        Ok(Self { env: env.clone(), collections })
    }
    
    pub fn collection<N: AsRef<str>>(&self, name: N) -> Result<Arc<Collection>, String> {
        let name = name.as_ref();

        {
            let collections = self.collections.read().wrap_err()?;
            if let Some(collection) = collections.iter().find(|collection| collection.name == name) {
                return Ok(collection.clone());
            }
        }
        
        let indexes = RwLock::new(Vec::new());
        let collection_db = Database::open(
            self.env.clone(), Some(&name), &DatabaseOptions::create_map::<[u8;8]>())
            .wrap_err()?;
        let env = self.env.clone();
        let collection = Arc::new(Collection { name: name.into(), indexes, env, db: Arc::new(collection_db) });

        {
            let mut collections = self.collections.write().wrap_err()?;
            collections.push(collection.clone());
        }
        
        Ok(collection)
    }
}
