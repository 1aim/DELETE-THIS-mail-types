#![recursion_limit="128"]

extern crate mail_codec_core as core;
extern crate mail_codec_headers as mheaders;

#[macro_use]
extern crate error_chain;

#[macro_use]
extern crate log;
extern crate mime;
extern crate futures;
extern crate rand;
extern crate soft_ascii_string;
extern crate total_order_multi_map;
extern crate tree_magic;
extern crate chrono;
extern crate vec1;




#[cfg(feature="default_impl_cpupool")]
extern crate futures_cpupool;

pub mod utils;
pub mod mail;
pub mod file_buffer;

#[cfg(feature="default_impl_any")]
pub mod default_impl;

pub use self::mail::*;

pub mod headers {
    pub use mheaders::*;
}

pub use mheaders::components::MediaType;

pub mod prelude {
    pub type Encoder = ::core::codec::Encoder<::mail::Resource>;
    pub use core::*;
    pub use core::error::*;
    pub use core::grammar::MailType;
    pub use core::codec::{EncodableInHeader, Encodable, EncodeHandle};
    pub use mheaders::*;
    pub use mheaders::components::*;
    pub use mail::Builder;
    pub use mail::mime::MultipartMime;
}