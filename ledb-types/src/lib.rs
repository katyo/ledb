extern crate serde;

#[macro_use]
extern crate serde_derive;

#[cfg(feature = "json")]
extern crate serde_json;

#[cfg(feature = "cbor")]
extern crate serde_cbor;

mod document;
mod identifier;

pub use self::document::*;
pub use self::identifier::*;
