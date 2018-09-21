use super::{Result, Storage};
use std::fs::remove_dir_all;

/*
macro_rules! json_str {
    ($($json:tt)+) => {
        to_string(&json!($($json)+)).unwrap()
    };
}
*/

macro_rules! json_val {
    ($($json:tt)+) => {
        from_value(json!($($json)+)).unwrap()
    };
}

macro_rules! test_parse {
    ($val_type:ty, $json_val:expr, $rust_val:expr) => {
        assert_eq!(
            from_str::<$val_type>(&to_string(&$json_val).unwrap()).unwrap(),
            $rust_val
        );
    };
}

macro_rules! test_build {
    ($rust_val:expr, $json_val:expr) => {
        assert_eq!(
            from_str::<Value>(&to_string(&$rust_val).unwrap()).unwrap(),
            from_str::<Value>(&to_string(&$json_val).unwrap()).unwrap()
        );
    };
}

static DB_DIR: &'static str = ".test_dbs";

pub fn test_db(id: &'static str) -> Result<Storage> {
    let path = format!("{}/{}", DB_DIR, id);

    let _ = remove_dir_all(&path);

    Storage::new(&path)
}
