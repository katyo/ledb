extern crate ledb_actix;
extern crate actix;
extern crate actix_web;

use ledb_actix::{Storage, storage};
use actix::{System};
use actix_web::{App, server};

fn main() {
    System::run(|| {
        let addr = Storage::new("database")
            .unwrap()
            .start(4);
        
        server::new(move || App::with_state(addr.clone())
                    .scope("/", storage))
            .bind("127.0.0.1:8888")
            .unwrap()
            .start();
    });
}
