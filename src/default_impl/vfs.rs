use std::path::{ PathBuf, Path };
use std::collections::HashMap;
use std::io;
use std::borrow::Cow;

use futures::future;

use core::error::*;
use context::FileLoader;

#[derive( Debug, Clone, PartialEq, Default )]
pub struct VFSFileLoader {
    files: HashMap<PathBuf, Vec<u8>>
}

impl VFSFileLoader {

    pub fn new() -> Self {
        VFSFileLoader { files: HashMap::new() }
    }

    /// registers a file to the file_loader part of test context,
    /// if a file under the given path already did exist, its
    /// content is overwritten and the old content is returned
    pub fn register_file<IP, IV>( &mut self,  path: IP, content: IV ) -> Result< Option< Vec<u8> > >
        where IP: Into<PathBuf>, IV: Into<Vec<u8>>
    {
        let path = path.into();
        let path = if path.is_relative() {
            Path::new( "/" ).join( path )
        } else { path };

        Ok( self.files.insert( path , content.into() ) )
    }
}


impl FileLoader for VFSFileLoader {
    type FileFuture = future::FutureResult<Vec<u8>, Error>;

    /// as we dont have to do "any blocking work" for loading from VFS,
    /// we can directly return a FutureResult
    fn load_file( &self, path: Cow<'static, Path> ) -> Self::FileFuture {
        let path = if path.is_relative() {
            Cow::Owned( Path::new( "/" ).join( path ) )
        } else { path };

        let result = if let Some( file ) = self.files.get( path.as_ref() ) {
            Ok( file.clone() )
        } else {
            let msg = path.to_string_lossy().into_owned();
            let err: Error = io::Error::new( io::ErrorKind::NotFound, msg ).into();
            Err( err )
        };
        future::result( result )
    }
}



#[cfg(test)]
mod test {
    use futures::Future;
    //use mail::FileLoader;
    use super::*;

    #[test]
    fn ok_if_registered() {
        let mut fl = VFSFileLoader::new();
        fl.register_file( "/my/resource.png", b"abcde".to_vec() ).unwrap();
        let res = fl.load_file( Cow::Owned( PathBuf::from( "/my/resource.png" ) ) ).wait();
        let buf = assert_ok!( res );
        assert_eq!(
            b"abcde".to_vec(),
            buf
        );
    }

    #[test]
    fn err_if_not_registered() {
        let mut fl = VFSFileLoader::new();
        fl.register_file( "/my/resource.png", b"abcde".to_vec() ).unwrap();
        let res = fl.load_file( Cow::Owned( PathBuf::from( "/not/my/resource.png" ) ) ).wait();
        assert_eq!( false, res.is_ok() );
    }

    #[test]
    fn handles_relative_paths_in_registry() {
        let mut fl = VFSFileLoader::new();
        fl.register_file( "./my/resource.png", b"abcde".to_vec() ).unwrap();
        let res = fl.load_file( Cow::Owned( PathBuf::from( "/my/resource.png" ) ) ).wait();
        let buf = assert_ok!( res );
        assert_eq!(
            b"abcde".to_vec(),
            buf
        );
    }

    #[test]
    fn handles_relative_paths_in_usage() {
        let mut fl = VFSFileLoader::new();
        fl.register_file( "/my/resource.png", b"abcde".to_vec() ).unwrap();
        let res = fl.load_file( Cow::Borrowed( &Path::new( "./my/resource.png" ) ) ).wait();
        let buf = assert_ok!( res );
        assert_eq!(
            b"abcde".to_vec(),
            buf
        );
    }

}