#![recursion_limit="128"]

#[macro_use]
extern crate log;
#[macro_use]
extern crate failure;
extern crate mime as media_type;
extern crate chrono;
#[macro_use]
extern crate futures;
#[cfg(feature="default_impl_cpupool")]
extern crate futures_cpupool;
extern crate rand;
extern crate vec1;
extern crate soft_ascii_string;

#[macro_use]
extern crate mail_common as common;
#[cfg_attr(test, macro_use)]
extern crate mail_headers as headers;


#[macro_use]
mod macros;
mod iri;
pub mod error;
pub mod utils;
pub mod file_buffer;
pub mod mime;
pub mod mail;
pub mod default_impl;


pub use self::iri::IRI;
pub use self::mail::*;
pub use ::context::Source;
