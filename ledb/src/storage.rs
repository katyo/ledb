use lmdb::{
    self, open::Flags as OpenFlags, Cursor, CursorIter, Database, DatabaseOptions, EnvBuilder,
    Environment, MaybeOwned, ReadTransaction,
};
use ron::de::from_str as from_db_name;
use std::collections::HashMap;
use std::fs::create_dir_all;
use std::path::Path;
use std::sync::{Arc, RwLock};

use super::{Collection, CollectionDef, IndexDef, Result, ResultWrap};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) enum DatabaseDef {
    #[serde(rename = "c")]
    Collection(CollectionDef),
    #[serde(rename = "i")]
    Index(IndexDef),
}

struct Collections {
    alive: Vec<Arc<Collection>>,
    dead: Vec<Arc<Collection>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Stats {
    pub page_size: u32,
    pub btree_depth: u32,
    pub branch_pages: usize,
    pub leaf_pages: usize,
    pub overflow_pages: usize,
    pub data_entries: usize,
}

impl From<lmdb::Stat> for Stats {
    fn from(
        lmdb::Stat {
            psize,
            depth,
            branch_pages,
            leaf_pages,
            overflow_pages,
            entries,
        }: lmdb::Stat,
    ) -> Self {
        Self {
            page_size: psize,
            btree_depth: depth,
            branch_pages,
            leaf_pages,
            overflow_pages,
            data_entries: entries,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Info {
    pub map_size: usize,
    pub last_page: usize,
    pub last_transaction: usize,
    pub max_readers: u32,
    pub num_readers: u32,
}

impl From<lmdb::EnvInfo> for Info {
    fn from(
        lmdb::EnvInfo {
            mapsize,
            last_pgno,
            last_txnid,
            maxreaders,
            numreaders,
            ..
        }: lmdb::EnvInfo,
    ) -> Self {
        Self {
            map_size: mapsize,
            last_page: last_pgno,
            last_transaction: last_txnid,
            max_readers: maxreaders,
            num_readers: numreaders,
        }
    }
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
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path = path.as_ref();
        let db_path = path.to_str().ok_or("Invalid db path").wrap_err()?;
        let mut bld = EnvBuilder::new().wrap_err()?;
        bld.set_maxdbs(1023).wrap_err()?;

        create_dir_all(path).wrap_err()?;
        let env = Arc::new(unsafe { bld.open(db_path, OpenFlags::empty(), 0o600) }.wrap_err()?);

        let db =
            Arc::new(Database::open(env.clone(), None, &DatabaseOptions::defaults()).wrap_err()?);

        let collections = RwLock::new(Collections {
            alive: load_databases(&env, &db)?
                .into_iter()
                .map(|(def, index_defs)| {
                    Collection::new(env.clone(), def, index_defs).map(Arc::new)
                })
                .collect::<Result<Vec<_>>>()?,
            dead: Vec::new(),
        });

        Ok(Self {
            env: env.clone(),
            collections,
        })
    }

    /// Check collection exists
    ///
    pub fn has_collection<N: AsRef<str>>(&self, name: N) -> Result<bool> {
        let name = name.as_ref();
        let collections = self.collections.read().wrap_err()?;
        // search alive collection
        Ok(collections
            .alive
            .iter()
            .any(|collection| collection.name == name))
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
            if let Some(collection) = collections
                .alive
                .iter()
                .find(|collection| collection.name == name)
            {
                return Ok(collection.clone());
            }
            // search dead collection
            collections
                .dead
                .iter()
                .position(|collection| collection.name == name)
        };

        // try to restore dead collection
        if let Some(pos) = dead_pos {
            // restore dead collection
            let mut collections = self.collections.write().wrap_err()?;
            let collection = collections.dead.remove(pos);
            collection.un_delete()?;
            collections.alive.push(collection.clone());
            return Ok(collection);
        }

        // create new collection
        let collection =
            Collection::new(self.env.clone(), CollectionDef::new(name), Vec::new()).map(Arc::new)?;

        let mut collections = self.collections.write().wrap_err()?;
        collections.alive.push(collection.clone());

        Ok(collection)
    }

    pub fn drop_collection<N: AsRef<str>>(&self, name: N) -> Result<bool> {
        let name = name.as_ref();

        let found_pos = {
            let collections = self.collections.read().wrap_err()?;
            collections
                .alive
                .iter()
                .position(|collection| collection.name == name)
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
        Ok(collections
            .alive
            .iter()
            .map(|collection| collection.name.clone())
            .collect())
    }

    pub fn get_stats(&self) -> Result<Stats> {
        self.env.stat().map(Stats::from).wrap_err()
    }

    pub fn get_info(&self) -> Result<Info> {
        self.env.info().map(Info::from).wrap_err()
    }
}

fn load_databases(env: &Environment, db: &Database) -> Result<Vec<(CollectionDef, Vec<IndexDef>)>> {
    let txn = ReadTransaction::new(env).wrap_err()?;
    let cursor = txn.cursor(db.clone()).wrap_err()?;
    let access = txn.access();
    let mut defs: HashMap<String, (CollectionDef, Vec<IndexDef>)> = HashMap::new();

    for res in CursorIter::new(
        MaybeOwned::Owned(cursor),
        &access,
        |c, a| c.first(a),
        Cursor::next::<str, [u8]>,
    ).wrap_err()?
        .map(|res| {
            res.wrap_err()
                .and_then(|(key, _val)| from_db_name(key).wrap_err())
        }) {
        match res {
            Ok(DatabaseDef::Collection(def)) => {
                defs.entry(def.0.clone())
                    .or_insert_with(|| (def, Vec::new()));
            }
            Ok(DatabaseDef::Index(def)) => defs
                .entry(def.0.clone())
                .or_insert_with(|| (CollectionDef::new(&def.0), Vec::new()))
                .1
                .push(def),
            Err(e) => return Err(e),
        }
    }

    Ok(defs.into_iter().map(|(_key, val)| val).collect())
}
