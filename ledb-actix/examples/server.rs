use actix_web::{middleware::Logger, App, HttpServer};
use ledb_actix::{storage, Options, Storage};
use std::env;

#[actix_rt::main]
async fn main() {
    env::set_var("RUST_LOG", "info");
    pretty_env_logger::init();

    let addr = Storage::new("database", Options::default())
        .unwrap()
        .start(4);

    let bind = "127.0.0.1:8888";

    HttpServer::new(move || {
        App::new()
            .wrap(Logger::default())
            .data(addr.clone())
            .service(storage())
    })
    .bind(&bind)
    .unwrap()
    .run()
    .await
    .unwrap();
}
