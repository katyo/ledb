use std::{
    collections::HashMap,
    env::current_dir,
    fs::create_dir_all,
    ops::Deref,
    path::{Path, PathBuf},
    sync::{Arc, RwLock},
};

use dirs::home_dir;
use dunce::canonicalize;
use lmdb::{
    self, open as OpenFlag, open::Flags as OpenFlags, Cursor, CursorIter, Database,
    DatabaseOptions, EnvBuilder, Environment, MaybeOwned, ReadTransaction,
};
use ron::de::from_str as from_db_name;
use serde::{Deserialize, Serialize};
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

/// Storage stats data
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

/// Storage info data
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

/// Database options
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Options {
    // options
    #[serde(default)]
    map_size: Option<usize>,
    #[serde(default)]
    max_readers: Option<u32>,
    #[serde(default)]
    max_dbs: Option<u32>,
    // flags
    #[serde(default)]
    map_async: Option<bool>,
    #[serde(default)]
    no_lock: Option<bool>,
    #[serde(default)]
    no_mem_init: Option<bool>,
    #[serde(default)]
    no_meta_sync: Option<bool>,
    #[serde(default)]
    no_read_ahead: Option<bool>,
    #[serde(default)]
    no_sub_dir: Option<bool>,
    #[serde(default)]
    no_sync: Option<bool>,
    #[serde(default)]
    no_tls: Option<bool>,
    #[serde(default)]
    read_only: Option<bool>,
    #[serde(default)]
    write_map: Option<bool>,
}

impl Options {
    fn env_builder(&self) -> Result<EnvBuilder> {
        let mut bld = EnvBuilder::new()?;

        bld.set_mapsize(self.map_size.unwrap_or(16 << 20))
            .wrap_err()?;
        bld.set_maxreaders(self.max_readers.unwrap_or(126))
            .wrap_err()?;
        bld.set_maxdbs(self.max_dbs.unwrap_or(128)).wrap_err()?;

        Ok(bld)
    }

    fn open_flags(&self) -> OpenFlags {
        self.fill_flags(None)
    }

    fn config_env(&self, env: &Environment) -> Result<()> {
        if let Some(val) = self.map_size {
            unsafe {
                env.set_mapsize(val).wrap_err()?;
            }
        }

        unsafe {
            env.set_flags(self.fill_flags(Some(true)), true)
                .wrap_err()?;
            env.set_flags(self.fill_flags(Some(false)), false)
                .wrap_err()?;
        }

        Ok(())
    }

    fn fill_flags(&self, onoff: Option<bool>) -> OpenFlags {
        let mut flags = OpenFlags::empty();

        if let Some(flag) = self.map_async {
            if onoff.map(|onoff| onoff == flag).unwrap_or(true) {
                flags.set(OpenFlag::MAPASYNC, flag);
            }
        }
        if let Some(flag) = self.no_lock {
            if onoff.map(|onoff| onoff == flag).unwrap_or(true) {
                flags.set(OpenFlag::NOLOCK, flag);
            }
        }
        if let Some(flag) = self.no_mem_init {
            if onoff.map(|onoff| onoff == flag).unwrap_or(true) {
                flags.set(OpenFlag::NOMEMINIT, flag);
            }
        }
        if let Some(flag) = self.no_meta_sync {
            if onoff.map(|onoff| onoff == flag).unwrap_or(true) {
                flags.set(OpenFlag::NOMETASYNC, flag);
            }
        }
        if let Some(flag) = self.no_read_ahead {
            if onoff.map(|onoff| onoff == flag).unwrap_or(true) {
                flags.set(OpenFlag::NORDAHEAD, flag);
            }
        }
        if let Some(flag) = self.no_sub_dir {
            if onoff.map(|onoff| onoff == flag).unwrap_or(true) {
                flags.set(OpenFlag::NOSUBDIR, flag);
            }
        }
        if let Some(flag) = self.no_sync {
            if onoff.map(|onoff| onoff == flag).unwrap_or(true) {
                flags.set(OpenFlag::NOSYNC, flag);
            }
        }
        if let Some(flag) = self.no_tls {
            if onoff.map(|onoff| onoff == flag).unwrap_or(true) {
                flags.set(OpenFlag::NOTLS, flag);
            }
        }
        if let Some(flag) = self.read_only {
            if onoff.map(|onoff| onoff == flag).unwrap_or(true) {
                flags.set(OpenFlag::RDONLY, flag);
            }
        }
        if let Some(flag) = self.write_map {
            if onoff.map(|onoff| onoff == flag).unwrap_or(true) {
                flags.set(OpenFlag::WRITEMAP, flag);
            }
        }

        flags
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
    /// Open documents storage using path to the database in filesystem
    ///
    /// When storage does not exists it will be created automatically.
    ///
    /// On opening storage the existing collections and indexes will be restored automatically.
    ///
    /// You can open multiple storages using same path, actually all of them will use same storage instance.
    /// Also you can clone storage instance, share it and and send it to another threads.
    ///
    pub fn new<P: AsRef<Path>>(path: P, opts: Options) -> Result<Self> {
        let path = realpath(path.as_ref())?;

        if let Some(storage) = Pool::get(&path)? {
            opts.config_env(&storage.env)?;
            Ok(Storage(storage))
        } else {
            Self::open(path, opts)
        }
    }

    fn open(path: PathBuf, opts: Options) -> Result<Self> {
        let env = open_env(&path, opts)?;

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

    /// Checks if the collection exists
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
    /// *Note*: The collection will be created automatically when is does not exists.
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

    /// Get openned storages
    pub fn openned() -> Result<Vec<PathBuf>> {
        Pool::lst()
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
        Supercow::shared(self)
    }
}

impl<'env> Into<NonSyncSupercow<'env, Environment>> for Storage {
    fn into(self) -> NonSyncSupercow<'env, Environment> {
        Supercow::shared(self)
    }
}

/// The list of collection and index definitions
type Definitions = Vec<(CollectionDef, Vec<IndexDef>)>;

fn load_databases(env: &Environment, db: &Database) -> Result<(Serial, Definitions)> {
    let txn = ReadTransaction::new(env).wrap_err()?;
    let cursor = txn.cursor(db).wrap_err()?;
    let access = txn.access();
    let mut defs: HashMap<String, (CollectionDef, Vec<IndexDef>)> = HashMap::new();
    let mut last_serial: Serial = 0;

    for res in CursorIter::new(
        MaybeOwned::Owned(cursor),
        &access,
        |c, a| c.first(a),
        Cursor::next::<str, [u8]>,
    )
    .wrap_err()?
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

fn open_env(path: &Path, opts: Options) -> Result<Environment> {
    let path = path.to_str().ok_or("Invalid db path").wrap_err()?;

    let bld = opts.env_builder()?;
    let flags = opts.open_flags();

    create_dir_all(&path).wrap_err()?;

    unsafe { bld.open(path, flags, 0o600) }.wrap_err()
}

fn realpath(path: &Path) -> Result<PathBuf> {
    let path = if path.has_root() {
        path.to_path_buf()
    } else if let Ok(path) = path.strip_prefix("~") {
        home_dir()
            .ok_or_else(|| "Unable to determine home directory")
            .wrap_err()?
            .as_path()
            .join(path)
    } else {
        current_dir().wrap_err()?.as_path().join(path)
    };
    safe_canonicalize(path.as_path())
}

fn safe_canonicalize(path: &Path) -> Result<PathBuf> {
    match canonicalize(path) {
        Ok(canonical) => Ok(canonical),
        Err(error) => {
            if let Some(parent) = path.parent() {
                let child = path.strip_prefix(parent).unwrap();
                safe_canonicalize(parent).map(|canonical_parent| canonical_parent.join(child))
            } else {
                Err(error).wrap_err()
            }
        }
    }
}
