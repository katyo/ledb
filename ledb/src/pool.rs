use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Once, RwLock, Weak, ONCE_INIT};

use super::{Result, ResultWrap, StorageData};

type Storages = Arc<RwLock<HashMap<PathBuf, Weak<StorageData>>>>;

static mut STORAGES: Option<Storages> = None;
static INITIALIZE_STORAGES: Once = ONCE_INIT;

#[inline]
fn init_storages() {
    INITIALIZE_STORAGES.call_once(|| unsafe {
        STORAGES = Some(Arc::new(RwLock::new(HashMap::new())));
    });
}

fn get_storages() -> Storages {
    init_storages();

    if let Some(storages) = unsafe { &STORAGES } {
        storages.clone()
    } else {
        unreachable!();
    }
}

pub(crate) struct Pool;

impl Pool {
    #[inline]
    pub(crate) fn get<P: AsRef<Path>>(path: P) -> Result<Option<Arc<StorageData>>> {
        let path = path.as_ref();
        let storages = get_storages();
        let map = storages.read().wrap_err()?;
        Ok(map.get(path).and_then(|env| env.upgrade()))
    }

    #[inline]
    pub(crate) fn put(path: PathBuf, storage: &Arc<StorageData>) -> Result<()> {
        let storages = get_storages();
        let mut map = storages.write().wrap_err()?;
        map.insert(path, Arc::downgrade(storage));
        Ok(())
    }

    #[inline]
    pub(crate) fn del<P: AsRef<Path>>(path: P) -> Result<()> {
        let path = path.as_ref();
        let storages = get_storages();
        let mut map = storages.write().wrap_err()?;
        map.remove(path);
        Ok(())
    }
}
