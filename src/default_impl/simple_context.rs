//! This module provides a type alias and constructor function for an simple context impl
//!
//! It used the `FsResourceLoader` and `CpuPool` with a `CompositeContext`.
//! Because of this it's only available if the features `default_impl_fs` and
//! `default_impl_cpupool` are set.
use std::io;

use soft_ascii_string::SoftAsciiString;
use futures_cpupool::{Builder, CpuPool};

use common::error::EncodingError;
use headers::components::Domain;

use ::context::CompositeContext;
use ::default_impl::{FsResourceLoader, HashedIdGen};

#[derive(Debug, Fail)]
pub enum ContextSetupError {
    #[fail(display="{}", _0)]
    ReadingEnv(io::Error),

    #[fail(display="{}", _0)]
    PunyCodingDomain(EncodingError)
}

pub type Context = CompositeContext<FsResourceLoader, CpuPool, HashedIdGen>;

/// create a new CompositeContext<FsResourceLoader, CpuPool, HashedIdGen>
///
/// It uses the current working directory as root for the `FsResourceLoader`,
/// and the default settings for the `CpuPool`, both the `domain` and
/// `unique_part` are passed to the `HashedIdGen::new` constructor.
pub fn new(domain: Domain, unique_part: SoftAsciiString) -> Result<Context, ContextSetupError> {
    let resource_loader = FsResourceLoader
        ::with_cwd_root()
        .map_err(|err| ContextSetupError::ReadingEnv(err))?;

    let cpu_pool = Builder::new().create();

    let id_gen = HashedIdGen
        ::new(domain, unique_part)
        .map_err(|err| ContextSetupError::PunyCodingDomain(err))?;

    Ok(CompositeContext::new(
        resource_loader,
        cpu_pool,
        id_gen,
    ))
}