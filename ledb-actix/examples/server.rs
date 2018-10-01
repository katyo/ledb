extern crate actix;
extern crate actix_web;
extern crate ledb_actix;
extern crate pretty_env_logger;

use actix::System;
use actix_web::{middleware::Logger, server, App};
use ledb_actix::{storage, Options, Storage};
use std::env;

fn main() {
    env::set_var("RUST_LOG", "info");
    pretty_env_logger::init().unwrap();

    System::run(|| {
        let addr = Storage::new("database", Options::default())
            .unwrap()
            .start(4);

        let bind = "127.0.0.1:8888";

        server::new(move || {
            App::with_state(addr.clone())
                .middleware(Logger::default())
                .scope("/", storage)
        }).bind(&bind)
        .unwrap()
        .start();
    });
}
