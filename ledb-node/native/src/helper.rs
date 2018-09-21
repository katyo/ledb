macro_rules! js_try {
    ($ctx:expr, $res:expr) => {
        match $res {
            Ok(val) => val,
            Err(err) => return $ctx.throw_error(format!("LEDB {}", err)),
        }
    };
}
