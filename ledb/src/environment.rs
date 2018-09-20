use lmdb::{open::Flags as OpenFlags, EnvBuilder, Environment};
use std::fs::create_dir_all;
use std::ops::Deref;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use supercow::{ext::ConstDeref, NonSyncSupercow, Supercow};

use super::{Pool, Result, ResultWrap};

#[derive(Clone)]
pub(crate) struct WrappedEnvironment {
    // Path
    path: PathBuf,
    // Environment handle
    env: Arc<Environment>,
}

impl WrappedEnvironment {
    pub(crate) fn new<P: AsRef<Path>>(path: P) -> Result<Self> {
        println!("Open db!: {:?}", path.as_ref().canonicalize());
        let path = safe_canonicalize(path.as_ref())?;

        let env = if let Some(env) = Pool::get_environment(&path)? {
            env
        } else {
            let db_path = path.to_str().ok_or("Invalid db path").wrap_err()?;

            println!("Open db: {}", &db_path);

            let mut bld = EnvBuilder::new().wrap_err()?;
            bld.set_maxdbs(1023).wrap_err()?;

            create_dir_all(&path).wrap_err()?;

            let env = Arc::new(unsafe { bld.open(db_path, OpenFlags::empty(), 0o600) }.wrap_err()?);
            Pool::put_environment(path.clone(), &env)?;
            env
        };

        Ok(Self { path, env })
    }
}

impl Drop for WrappedEnvironment {
    fn drop(&mut self) {
        if let Err(e) = Pool::del_environment(&self.path) {
            eprintln!("Error when deleting environment: {}", e);
        }
    }
}

impl Deref for WrappedEnvironment {
    type Target = Environment;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &*self.env
    }
}

unsafe impl ConstDeref for WrappedEnvironment {
    type Target = Environment;

    #[inline]
    fn const_deref(&self) -> &Self::Target {
        &*self.env
    }
}

impl<'env> Into<Supercow<'env, Environment>> for WrappedEnvironment {
    fn into(self) -> Supercow<'env, Environment> {
        let this = self.clone();
        Supercow::shared(this)
    }
}

impl<'env> Into<NonSyncSupercow<'env, Environment>> for WrappedEnvironment {
    fn into(self) -> NonSyncSupercow<'env, Environment> {
        let this = self.clone();
        Supercow::shared(this)
    }
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

#[cfg(test)]
mod test {
    use super::WrappedEnvironment;
    use lmdb::{Database, DatabaseOptions, ReadTransaction};

    #[test]
    #[ignore]
    fn test() {
        let env = WrappedEnvironment::new("wrapped-env-db").unwrap();

        let _db = Database::open(env.clone(), None, &DatabaseOptions::defaults()).unwrap();
        let _txn = ReadTransaction::new(env.clone()).unwrap();
    }
}
