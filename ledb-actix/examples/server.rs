extern crate actix;
extern crate actix_web;
extern crate ledb_actix;
extern crate pretty_env_logger;

use actix::System;
use actix_web::{middleware::Logger, App, HttpServer};
use ledb_actix::{storage, Options, Storage};
use std::env;

fn main() {
    env::set_var("RUST_LOG", "info");
    pretty_env_logger::init();

    System::run(|| {
        let addr = Storage::new("database", Options::default())
            .unwrap()
            .start(4);

        let bind = "127.0.0.1:8888";

        HttpServer::new(move || {
            App::new()
                .wrap(Logger::default())
                .data(addr.clone())
                .service(storage())
        }).bind(&bind)
        .unwrap()
        .start();
    }).unwrap();
}
