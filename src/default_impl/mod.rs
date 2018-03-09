
#[cfg(test)]
use context::CompositeBuilderContext;
#[cfg(test)]
use futures_cpupool as _cpupool;


#[cfg(feature="default_impl_cpupool")]
mod cpupool;
#[cfg(feature="default_impl_cpupool")]
pub use self::cpupool::*;


#[cfg(feature="default_impl_fs")]
mod fs;
#[cfg(feature="default_impl_fs")]
pub use self::fs::*;


#[cfg(all(feature="default_impl_fs", feature="default_impl_cpupool"))]
pub mod simple_context;

#[cfg(all(
    test,
    not(feature="default_impl_cpupool"),
    not(feature="default_impl_fs")))]
compile_error!("test need following (default) features: default_impl_cpupool, default_impl_fs");

#[cfg(test)]
pub type TestContext = CompositeBuilderContext<FsResourceLoader, _cpupool::CpuPool>;

//same crate so we can do this ;=)
#[cfg(test)]
pub fn test_context() -> TestContext {
    //TODO crate a test context which does not access the file system
    CompositeBuilderContext::new(
        FsResourceLoader::with_cwd_root().unwrap(),
        _cpupool::Builder::new().create()
    )
}




