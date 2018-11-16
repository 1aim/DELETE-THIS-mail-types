//! Provides the core mail type `Mail` for the `mail` crate.
//! This crate provides the type called `mail` as well as ways
//! to create it. It also provides the builder context interface
//! and the `Resource` type, which is used to represent mail bodies.
//! Especially such which are attachments or embedded images.
//!
#![recursion_limit="128"]

#[macro_use]
extern crate log;
#[macro_use]
extern crate failure;
extern crate mime as media_type;
extern crate chrono;
extern crate futures;
extern crate rand;
extern crate vec1;
extern crate soft_ascii_string;

#[cfg(feature="serde")]
extern crate serde;
#[cfg(all(test, feature="serde"))]
extern crate serde_test;

#[cfg(feature="default_impl_cpupool")]
extern crate futures_cpupool;

extern crate mail_internals as common;
#[cfg_attr(test, macro_use)]
extern crate mail_headers as headers;
extern crate checked_command;


#[macro_use]
mod macros;
mod iri;
pub mod error;
pub mod utils;
pub mod mime;
pub mod context;
mod resource;
mod encode;
mod mail;
pub mod compose;

pub mod default_impl;

pub use self::iri::IRI;
pub use self::resource::*;
pub use self::mail::*;

pub use ::context::Context;



#[cfg(all(feature="serde", not(feature="serde-impl")))]
compile_error!(concat!(
    "\n---------------------------------------\n",
    " for serde use feature `serde-impl`,\n",
    " `serde` can not be used as feature in\n",
    " this crate due to limitations with Cargo\n",
    "-----------------------------------------\n"
));