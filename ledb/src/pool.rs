use lmdb::Environment;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Once, RwLock, Weak, ONCE_INIT};

use super::{Result, ResultWrap};

type Environments = Arc<RwLock<HashMap<PathBuf, Weak<Environment>>>>;

static mut ENVIRONMENTS: Option<Environments> = None;
static INITIALIZE_ENVIRONMENTS: Once = ONCE_INIT;

#[inline]
fn init_environments() {
    INITIALIZE_ENVIRONMENTS.call_once(|| unsafe {
        ENVIRONMENTS = Some(Arc::new(RwLock::new(HashMap::new())));
    });
}

fn get_environments() -> Environments {
    init_environments();

    if let Some(environments) = unsafe { &ENVIRONMENTS } {
        environments.clone()
    } else {
        unreachable!();
    }
}

pub(crate) struct Pool;

impl Pool {
    #[inline]
    pub(crate) fn get_environment<P: AsRef<Path>>(path: P) -> Result<Option<Arc<Environment>>> {
        let path = path.as_ref();
        let environments = get_environments();
        let map = environments.read().wrap_err()?;
        Ok(map.get(path).and_then(|env| env.upgrade()))
    }

    #[inline]
    pub(crate) fn put_environment(path: PathBuf, env: &Arc<Environment>) -> Result<()> {
        let environments = get_environments();
        let mut map = environments.write().wrap_err()?;
        map.insert(path, Arc::downgrade(env));
        Ok(())
    }

    #[inline]
    pub(crate) fn del_environment<P: AsRef<Path>>(path: P) -> Result<()> {
        let path = path.as_ref();
        let environments = get_environments();
        let mut map = environments.write().wrap_err()?;
        map.remove(path);
        Ok(())
    }
}
