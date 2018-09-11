macro_rules! json_str {
    ($($json:tt)+) => {
        to_string(&json!($($json)+)).unwrap()
    };
}

macro_rules! json_val {
    ($($json:tt)+) => {
        from_value(json!($($json)+)).unwrap()
    };
}

macro_rules! test_parse {
    ($val_type:ty, $json_val:expr, $rust_val:expr) => {
        assert_eq!(from_str::<$val_type>(&to_string(&$json_val).unwrap()).unwrap(),
                   $rust_val);
    }
}

macro_rules! test_build {
    ($rust_val:expr, $json_val:expr) => {
        assert_eq!(to_string(&$rust_val).unwrap(),
                   to_string(&$json_val).unwrap());
    }
}
