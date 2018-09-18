use std::fs::create_dir_all;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use ron::de::from_str as from_db_name;
use lmdb::{EnvBuilder, Environment, open::Flags as OpenFlags, Database, DatabaseOptions, ReadTransaction, Cursor, CursorIter, MaybeOwned};

use super::{Result, ResultWrap, CollectionDef, Collection, IndexDef};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) enum DatabaseDef {
    #[serde(rename="c")]
    Collection(CollectionDef),
    #[serde(rename="i")]
    Index(IndexDef),
}

struct Collections {
    alive: Vec<Arc<Collection>>,
    dead: Vec<Arc<Collection>>,
}

/// Storage of documents
pub struct Storage {
    env: Arc<Environment>,
    collections: RwLock<Collections>,
}

impl Storage {
    /// Open documents storage
    ///
    /// When storage does not exists it will be created automatically.
    ///
    /// On opening storage the existing collections and indexes will be restored automatically.
    ///
    pub fn open<P: AsRef<str>>(path: P) -> Result<Self> {
        let path = path.as_ref();
        let mut bld = EnvBuilder::new().wrap_err()?;
        bld.set_maxdbs(1023).wrap_err()?;

        create_dir_all(path).wrap_err()?;
        let env = Arc::new(unsafe { bld.open(path, OpenFlags::empty(), 0o600) }
        .wrap_err()?);

        let db = Arc::new(Database::open(
            env.clone(), None, &DatabaseOptions::defaults())
                          .wrap_err()?);

        let collections = RwLock::new(Collections {
            alive: load_databases(&env, &db)?
                .into_iter()
                .map(|(def, index_defs)|
                     Collection::new(env.clone(), def, index_defs)
                     .map(Arc::new))
                .collect::<Result<Vec<_>>>()?,
            dead: Vec::new(),
        });
        
        Ok(Self { env: env.clone(), collections })
    }

    /// Get collection for documents
    ///
    /// *Note*: The collection will be created automatically when does not exists.
    ///
    pub fn collection<N: AsRef<str>>(&self, name: N) -> Result<Arc<Collection>> {
        let name = name.as_ref();

        let dead_pos = {
            let collections = self.collections.read().wrap_err()?;
            // search alive collection
            if let Some(collection) = collections.alive.iter().find(|collection| collection.name == name) {
                return Ok(collection.clone());
            }
            // search dead collection
            collections.dead.iter().position(|collection| collection.name == name)
        };

        // try to restore dead collection
        if let Some(pos) = dead_pos {
            // restore dead collection
            let mut collections = self.collections.write().wrap_err()?;
            let collection = collections.dead.remove(pos);
            collections.alive.push(collection.clone());
            return Ok(collection);
        }

        // create new collection
        let collection = Collection::new(self.env.clone(),
                                         CollectionDef::new(name),
                                         Vec::new())
            .map(Arc::new)?;

        let mut collections = self.collections.write().wrap_err()?;
        collections.alive.push(collection.clone());
        
        Ok(collection)
    }

    pub fn drop_collection<N: AsRef<str>>(&self, name: N) -> Result<bool> {
        let name = name.as_ref();
        
        let found_pos = {
            let collections = self.collections.read().wrap_err()?;
            collections.alive.iter().position(|collection| collection.name == name)
        };

        Ok(if let Some(pos) = found_pos {
            let mut collections = self.collections.write().wrap_err()?;
            let collection = collections.alive.remove(pos);
            collection.to_delete()?;
            collections.dead.push(collection);
            true
        } else {
            false
        })
    }

    pub fn get_collections(&self) -> Result<Vec<String>> {
        let collections = self.collections.read().wrap_err()?;
        Ok(collections.alive.iter().map(|collection| collection.name.clone()).collect())
    }
}

impl Drop for Storage {
    fn drop(&mut self) {
        let mut collections = self.collections.write().unwrap();

        for collection in collections.dead.drain(..) {
            if let Ok(collection) = Arc::try_unwrap(collection) {
                collection.do_delete().unwrap();
            }
        }
    }
}

fn load_databases(env: &Environment, db: &Database) -> Result<Vec<(CollectionDef, Vec<IndexDef>)>> {
    let txn = ReadTransaction::new(env).wrap_err()?;
    let cursor = txn.cursor(db.clone()).wrap_err()?;
    let access = txn.access();
    let mut defs: HashMap<String, (CollectionDef, Vec<IndexDef>)> = HashMap::new();
    
    for res in CursorIter::new(MaybeOwned::Owned(cursor), &access,
                    |c, a| c.first(a), Cursor::next::<str,[u8]>)
        .wrap_err()?
        .map(|res| res.wrap_err().and_then(|(key, _val)| from_db_name(key).wrap_err())) {
            match res {
                Ok(DatabaseDef::Collection(def)) => {
                    defs.entry(def.0.clone())
                        .or_insert_with(|| (def, Vec::new()));
                },
                Ok(DatabaseDef::Index(def)) => {
                    defs.entry(def.0.clone())
                        .or_insert_with(|| (CollectionDef::new(&def.0), Vec::new()))
                        .1.push(def)
                },
                Err(e) => return Err(e),
            }
        }
    
    Ok(defs.into_iter().map(|(_key, val)| val).collect())
}
