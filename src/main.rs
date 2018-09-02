extern crate bytes;
extern crate serde;
#[macro_use] extern crate serde_derive;
extern crate serde_cbor;
extern crate serde_json;
extern crate lmdb_zero as lmdb;

mod key;
mod val;

use std::fs::create_dir_all;

use key::{IntoKey, FromKey};
use val::{IntoVal, FromVal};

pub type UserId = u32;

#[derive(Debug, Serialize, Deserialize)]
pub struct UserData {
    pub id: Option<UserId>,
    pub name: String,
    pub hash: Option<Vec<u8>>,
    pub prof: UserProf,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UserProf {
    pub oses: Vec<String>,
    pub langs: Vec<String>,
}

fn main() {
    println!("Hello, world!");

    let mut bld = lmdb::EnvBuilder::new().unwrap();
    bld.set_maxdbs(1000).unwrap();

    create_dir_all("db").unwrap();
    let env = unsafe { bld.open("db", lmdb::open::Flags::empty(), 0o600) }.unwrap();
    
    // Open the database.
    let user_by_id = lmdb::Database::open(
        &env, Some("user_by_id"), &lmdb::DatabaseOptions::create_map::<str>())
        .unwrap();
    
    {
        let txn = lmdb::WriteTransaction::new(&env).unwrap();
        let f = lmdb::put::Flags::empty();

        {
            let mut access = txn.access();
            access.put(&user_by_id, &1u32.into_key(), &UserData { id: Some(1), name: "kayo".into(), hash: None, prof: UserProf { oses: vec![], langs: vec!["rust".into()] } }.into_val(), f).unwrap();
        }
        
        txn.commit().unwrap();
    }
}
