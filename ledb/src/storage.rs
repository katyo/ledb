use lmdb::{
    self, open::Flags as OpenFlags, Cursor, CursorIter, Database, DatabaseOptions, EnvBuilder,
    Environment, MaybeOwned, ReadTransaction,
};
use ron::de::from_str as from_db_name;
use std::collections::HashMap;
use std::fs::create_dir_all;
use std::ops::Deref;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};
use supercow::{ext::ConstDeref, NonSyncSupercow, Supercow};

use super::{
    Collection, CollectionDef, Enumerable, IndexDef, Pool, Result, ResultWrap, Serial,
    SerialGenerator,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) enum DatabaseDef {
    #[serde(rename = "c")]
    Collection(CollectionDef),
    #[serde(rename = "i")]
    Index(IndexDef),
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

pub(crate) struct StorageData {
    path: PathBuf,
    env: Environment,
    gen: SerialGenerator,
    collections: RwLock<Vec<Collection>>,
}

/// Storage of documents
#[derive(Clone)]
pub struct Storage(Arc<StorageData>);

impl Storage {
    /// Open documents storage
    ///
    /// When storage does not exists it will be created automatically.
    ///
    /// On opening storage the existing collections and indexes will be restored automatically.
    ///
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path = safe_canonicalize(path.as_ref())?;

        if let Some(storage) = Pool::get(&path)? {
            Ok(Storage(storage))
        } else {
            Self::open(path)
        }
    }

    fn open(path: PathBuf) -> Result<Self> {
        let env = open_env(&path)?;

        let gen = SerialGenerator::new();

        let collections = RwLock::new(Vec::new());

        let storage = Storage(Arc::new(StorageData {
            path: path.clone(),
            env,
            gen,
            collections,
        }));

        storage.load_collections()?;

        Pool::put(path, &storage.0)?;

        Ok(storage)
    }

    fn load_collections(&self) -> Result<()> {
        let env = &self.0.env;

        let db = Database::open(env, None, &DatabaseOptions::defaults()).wrap_err()?;

        let (last_serial, db_def) = load_databases(&env, &db)?;

        self.0.gen.set(last_serial);

        let mut collections = self.0.collections.write().wrap_err()?;

        *collections = db_def
            .into_iter()
            .map(|(def, index_defs)| Collection::new(self.clone(), def, index_defs))
            .collect::<Result<Vec<_>>>()?;

        Ok(())
    }

    pub(crate) fn enumerate<E: Enumerable>(&self, data: E) -> E {
        self.0.gen.enumerate(data)
    }

    /// Check collection exists
    ///
    pub fn has_collection<N: AsRef<str>>(&self, name: N) -> Result<bool> {
        let name = name.as_ref();
        let collections = self.0.collections.read().wrap_err()?;
        // search alive collection
        Ok(collections
            .iter()
            .any(|collection| collection.name() == name))
    }

    /// Get collection for documents
    ///
    /// *Note*: The collection will be created automatically when does not exists.
    ///
    pub fn collection<N: AsRef<str>>(&self, name: N) -> Result<Collection> {
        let name = name.as_ref();

        {
            let collections = self.0.collections.read().wrap_err()?;
            // search alive collection
            if let Some(collection) = collections
                .iter()
                .find(|collection| collection.name() == name)
            {
                return Ok(collection.clone());
            }
        }

        // create new collection
        let collection = Collection::new(
            self.clone(),
            self.enumerate(CollectionDef::new(name)),
            Vec::new(),
        )?;

        let mut collections = self.0.collections.write().wrap_err()?;
        collections.push(collection.clone());

        Ok(collection)
    }

    pub fn drop_collection<N: AsRef<str>>(&self, name: N) -> Result<bool> {
        let name = name.as_ref();

        let found_pos = {
            let collections = self.0.collections.read().wrap_err()?;
            collections
                .iter()
                .position(|collection| collection.name() == name)
        };

        Ok(if let Some(pos) = found_pos {
            let mut collections = self.0.collections.write().wrap_err()?;
            let collection = collections.remove(pos);
            collection.to_delete()?;
            true
        } else {
            false
        })
    }

    pub fn get_collections(&self) -> Result<Vec<String>> {
        let collections = self.0.collections.read().wrap_err()?;
        Ok(collections
            .iter()
            .map(|collection| collection.name().into())
            .collect())
    }

    pub fn get_stats(&self) -> Result<Stats> {
        self.0.env.stat().map(Stats::from).wrap_err()
    }

    pub fn get_info(&self) -> Result<Info> {
        self.0.env.info().map(Info::from).wrap_err()
    }
}

impl Drop for Storage {
    fn drop(&mut self) {
        if let Err(e) = Pool::del(&self.0.path) {
            eprintln!("Error when dropping storage: {}", e);
        }
    }
}

impl Deref for Storage {
    type Target = Environment;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.0.env
    }
}

unsafe impl ConstDeref for Storage {
    type Target = Environment;

    #[inline]
    fn const_deref(&self) -> &Self::Target {
        &self.0.env
    }
}

impl<'env> Into<Supercow<'env, Environment>> for Storage {
    fn into(self) -> Supercow<'env, Environment> {
        let this = self.clone();
        Supercow::shared(this)
    }
}

impl<'env> Into<NonSyncSupercow<'env, Environment>> for Storage {
    fn into(self) -> NonSyncSupercow<'env, Environment> {
        let this = self.clone();
        Supercow::shared(this)
    }
}

fn load_databases(
    env: &Environment,
    db: &Database,
) -> Result<(Serial, Vec<(CollectionDef, Vec<IndexDef>)>)> {
    let txn = ReadTransaction::new(env).wrap_err()?;
    let cursor = txn.cursor(db.clone()).wrap_err()?;
    let access = txn.access();
    let mut defs: HashMap<String, (CollectionDef, Vec<IndexDef>)> = HashMap::new();
    let mut last_serial: Serial = 0;

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
                last_serial = usize::max(last_serial, def.0);
                let entry = defs
                    .entry(def.1.clone())
                    .or_insert_with(|| (def.clone(), Vec::new()));
                entry.0 = def;
            }
            Ok(DatabaseDef::Index(def)) => {
                last_serial = usize::max(last_serial, def.0);
                defs.entry(def.1.clone())
                    .or_insert_with(|| (CollectionDef::new(&def.1), Vec::new()))
                    .1
                    .push(def);
            }
            Err(e) => return Err(e),
        }
    }

    Ok((
        last_serial,
        defs.into_iter().map(|(_key, val)| val).collect(),
    ))
}

fn open_env(path: &Path) -> Result<Environment> {
    let db_path = path.to_str().ok_or("Invalid db path").wrap_err()?;

    let mut bld = EnvBuilder::new().wrap_err()?;
    bld.set_maxdbs(1023).wrap_err()?;

    create_dir_all(&path).wrap_err()?;

    unsafe { bld.open(db_path, OpenFlags::empty(), 0o600) }.wrap_err()
}

fn safe_canonicalize(path: &Path) -> Result<PathBuf> {
    match path.canonicalize() {
        Ok(canonical) => Ok(canonical),
        Err(error) => if let Some(parent) = path.parent() {
            let child = path.strip_prefix(parent).unwrap();
            safe_canonicalize(parent).map(|parent| parent.join(child))
        } else {
            Err(error).wrap_err()
        },
    }
}
