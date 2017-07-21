extern crate ascii;
extern crate mime;
extern crate owning_ref;
extern crate quoted_printable;
extern crate chrono;
extern crate futures;
extern crate serde;
extern crate base64;
#[macro_use]
extern crate lazy_static;

#[macro_use]
extern crate serde_derive;

#[macro_use]
extern crate nom;

#[macro_use]
extern crate error_chain;

#[macro_use]
mod macros;

pub mod error;
pub mod raw_mail;
pub mod mail_composition;

pub mod codec;
pub mod headers;
pub mod types;

mod utils;
