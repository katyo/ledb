use std::{
    fs::remove_dir_all,
    path::Path,
};

use super::{Options, Result, Storage};

macro_rules! json_val {
    ($($json:tt)+) => {
        from_value(serde_json::json!($($json)+)).unwrap()
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

static DB_DIR: &'static str = "test_db";

pub fn test_db(id: &'static str) -> Result<Storage> {
    let path = Path::new(DB_DIR).join(Path::new(id));

    let _ = remove_dir_all(&path);

    Storage::new(&path, Options::default())
}
