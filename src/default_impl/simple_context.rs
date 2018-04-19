//! This module provides a type alias and constructor function for an simple context impl
//!
//! It used the `FsResourceLoader` and `CpuPool` with a `CompositeBuilderContext`.
//! Because of this it's only available if the features `default_impl_fs` and
//! `default_impl_cpupool` are set.
use std::io;

use futures_cpupool::{Builder, CpuPool};

use ::context::CompositeBuilderContext;
use ::default_impl::FsResourceLoader;

pub type Context = CompositeBuilderContext<FsResourceLoader, CpuPool>;

/// create a new CompositeBuilderContext<FsResourceLoader, CpuPool>
///
/// It uses the current working directory as root for the `FsResourceLoader`,
/// and the default settings for the `CpuPool`.
pub fn new() -> Result<Context, io::Error> {
    Ok(CompositeBuilderContext::new(
        FsResourceLoader::with_cwd_root()?,
        Builder::new().create()
    ))
}