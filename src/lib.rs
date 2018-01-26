#![recursion_limit="128"]

#[macro_use]
extern crate mail_codec_core as core;
extern crate mail_codec_headers as mheaders;

#[macro_use]
extern crate error_chain;

#[macro_use]
extern crate log;
extern crate mime;
extern crate futures;
extern crate serde;
extern crate rand;
extern crate soft_ascii_string;
extern crate total_order_multi_map;
extern crate tree_magic;
extern crate chrono;
extern crate vec1;

#[macro_use]
extern crate serde_derive;

#[macro_use]
extern crate scoped_tls;


#[cfg(feature="default_impl_cpupool")]
extern crate futures_cpupool;

pub mod utils;
pub mod mail;
pub mod composition;
pub mod file_buffer;

#[cfg(feature="default_impl_any")]
pub mod default_impl;

pub use self::mail::*;

pub mod headers {
    pub use mheaders::*;
}

pub use mheaders::components::MediaType;

pub mod mail_builder_prelude {
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


pub mod resource_prelude {
    pub use file_buffer::FileBuffer;
    pub use core::utils::FileMeta;
    pub use mail::{ Resource, ResourceSpec };
    pub use composition::{ Embedding, Attachment, EmbeddingWithCID };
}

pub mod composition_prelude {
    pub type Encoder = ::core::codec::Encoder<::mail::Resource>;
    pub use core::*;
    pub use core::error::*;
    pub use core::grammar::MailType;
    pub use mheaders::components::{
        Mailbox,
        Email,
        TransferEncoding,
        MediaType
    };
    pub use core::codec::{
        EncodableInHeader,
        EncodeHandle,
        Encodable
    };
    pub use composition::{
        Compositor,
        NameComposer,
        MailSendContext,
    };
}

pub mod template_engine_prelude {
    pub type StdError = ::std::error::Error;
    pub type StdResult<R,E> = ::std::result::Result<R,E>;
    pub use serde::Serialize;

    pub use vec1::Vec1;
    pub use mail::{
        Resource
    };
    pub use composition::{
        Template, TemplateEngine,
        Context,
        Attachment, Embedding
    };
 }
