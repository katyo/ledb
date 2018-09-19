extern crate ledb_actix;
extern crate actix;
extern crate actix_web;
extern crate pretty_logger;

use ledb_actix::{Storage, storage};
use actix::{System};
use actix_web::{App, server, middleware::Logger};

fn main() {
    pretty_logger::init_to_defaults().unwrap();
    
    System::run(|| {
        let addr = Storage::new("database")
            .unwrap()
            .start(4);

        let bind = "127.0.0.1:8888";
        
        server::new(move || App::with_state(addr.clone())
                    .middleware(Logger::default())
                    .scope("/", storage))
            .bind(&bind)
            .unwrap()
            .start();
    });
}
