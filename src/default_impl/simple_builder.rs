use std::sync::Arc;
use std::default::Default;
use std::path::Path;
use std::fmt;
use std::borrow::Cow;

use futures::Future;
use futures_cpupool::{ CpuPool, Builder };

use utils::SendBoxFuture;
use context::{ FileLoader, RunElsewhere, CompositeBuilderContext };
use super::VFSFileLoader;

#[derive(Clone)]
pub struct SimpleBuilderContext( Arc< CompositeBuilderContext<VFSFileLoader, CpuPool>  > );



impl fmt::Debug for SimpleBuilderContext {
    fn fmt( &self, fter: &mut fmt::Formatter ) -> fmt::Result {
        fter.debug_struct( "SimpleBuilderContext" )
            .field( "file_loader", &self.0.file_loader )
            .field( "elsewher", &"CpuPool { .. }" )
            .finish()
    }
}

impl Default for SimpleBuilderContext {
    fn default() -> Self {
        SimpleBuilderContext::new()
    }
}

impl SimpleBuilderContext {

    pub fn new() -> Self {
        Self::with_cpupool( Builder::new() )
    }

    pub fn with_vfs( vfs: VFSFileLoader ) -> Self {
        Self::with_vfs_and_cpupool( vfs, Builder::new() )
    }

    pub fn with_cpupool( cpupool: Builder ) -> Self {
        Self::with_vfs_and_cpupool( VFSFileLoader::new(), cpupool )
    }

    pub fn with_vfs_and_cpupool( vfs: VFSFileLoader, mut cpupool: Builder ) -> Self {
        SimpleBuilderContext( Arc::new( CompositeBuilderContext::new(
            vfs,
            cpupool.create()
        ) ) )
    }

}


impl FileLoader for SimpleBuilderContext {
    type FileFuture = <VFSFileLoader as FileLoader>::FileFuture;
    fn load_file( &self, path: Cow<'static, Path> ) -> Self::FileFuture {
        self.0.load_file( path )
    }
}

impl RunElsewhere for SimpleBuilderContext {
    fn execute<F>( &self, fut: F) -> SendBoxFuture<F::Item, F::Error>
        where F: Future + Send + 'static,
              F::Item: Send+'static,
              F::Error: Send+'static
    {
        self.0.execute( fut )
    }
}