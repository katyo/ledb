extern crate serde;
extern crate ledb;
extern crate actix;

#[cfg(any(test, feature = "web"))]
extern crate futures;

#[cfg(test)]
extern crate tokio;

#[cfg(any(test, feature = "web"))]
#[macro_use]
extern crate serde_derive;

#[cfg(test)]
#[macro_use]
extern crate serde_json;

#[cfg(feature = "web")]
extern crate serde_with;

#[cfg(feature = "web")]
extern crate actix_web;

mod actor;

#[cfg(feature = "web")]
mod scope;

pub use actor::*;

#[cfg(feature = "web")]
pub use scope::*;
