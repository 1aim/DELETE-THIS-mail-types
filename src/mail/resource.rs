use std::marker::PhantomData;
use std::path::PathBuf;
use std::fmt;
use std::sync::{ Arc, RwLock, RwLockWriteGuard, RwLockReadGuard };
use std::ops::Deref;
use std::mem;
use std::borrow::Cow;

use mime::{TEXT, APPLICATION, OCTET_STREAM};
use tree_magic;
use futures::{  Future, Poll, Async };

use core::error::{ Error, ErrorKind, Result, ResultExt };
use core::codec::BodyBuffer;
use core::utils::FileMeta;

use mheaders::components::{MediaType, TransferEncoding};


use utils::{SendBoxFuture, now};
use file_buffer::{FileBuffer, TransferEncodedFileBuffer};
use super::context::BuilderContext;


/// POD containing the path from which the resource should be loaded as well as mime and name
/// if no mime is specified, the mime is sniffed if possible
/// if no name is specified the base name of the path is used
#[derive( Debug, Clone )]
pub struct ResourceSpec {
    pub path: PathBuf,
    pub use_name: Option<String>,
    pub use_mime: Option<MediaType>
}

#[derive(Debug)]
pub struct ResourceFutureRef<'a, C: 'a> {
    resource_ref: &'a mut Resource,
    ctx_ref: &'a C
}

#[derive( Debug, Clone )]
pub struct Resource {
    inner: Arc<RwLock<ResourceInner>>,
    preferred_encoding: Option<TransferEncoding>
}

#[derive(Debug, Eq, PartialEq, Hash, Clone, Copy)]
pub enum ResourceState {
    HasSpec,
    LoadingFileBuffer,
    LoadedFileBuffer,
    EncodingFileBuffer,
    EncodedFileBuffer,
    HadError
}

enum ResourceInner {
    Spec( ResourceSpec ),
    LoadingBuffer( SendBoxFuture<FileBuffer, Error> ),
    Loaded( FileBuffer ),
    EncodingBuffer( SendBoxFuture<TransferEncodedFileBuffer, Error> ),
    TransferEncoded( TransferEncodedFileBuffer ),
    Failed
}

impl ResourceInner {

    fn state(&self) -> ResourceState {
        use self::ResourceInner::*;
        use self::ResourceState::*;
        match *self {
            Spec(..) => HasSpec,
            LoadingBuffer(..) => LoadingFileBuffer,
            Loaded(..) => LoadedFileBuffer,
            EncodingBuffer(..) => EncodingFileBuffer,
            TransferEncoded(..) => EncodedFileBuffer,
            Failed => HadError
        }
    }

}

pub struct Guard<'lock> {
    //NOTE: this is NOT dead_code (field never used),
    // just unused through it still _drops_ and has a _side effect_
    // on drop (which is what rustc's lint does not "know")
    #[allow(dead_code)]
    guard: RwLockReadGuard<'lock, ResourceInner>,
    inner_ref: *const TransferEncodedFileBuffer,
    // given that we neither own a value we point to (DropCheck) nor
    // have a unused type parameter nor lifetime this is probably not
    // needed, still it's better to be safe and have this zero-runtime-overhead
    // marker
    _marker: PhantomData<&'lock TransferEncodedFileBuffer>
}



impl Resource {

    pub fn from_text(text: String ) -> Self {
        //UNWRAP_SAFE: this is a valid mime, if not this will be coucht by the tests
        let content_type = MediaType::parse("text/plain;charset=utf8").unwrap();
        let buf = FileBuffer::new( content_type, text.into_bytes() );
        Resource::from_buffer( buf )
    }

    #[inline]
    pub fn from_spec( spec: ResourceSpec ) -> Self {
        Self::new_inner( ResourceInner::Spec( spec ) )
    }

    #[inline]
    pub fn from_buffer( buffer: FileBuffer ) -> Self {
        Self::new_inner( ResourceInner::Loaded( buffer ) )
    }

    #[inline]
    pub fn from_future( fut: SendBoxFuture<FileBuffer, Error> ) -> Self {
        Self::new_inner( ResourceInner::LoadingBuffer( fut ) )
    }

    #[inline]
    pub fn from_encoded_buffer( buffer: TransferEncodedFileBuffer ) -> Self {
        Self::new_inner( ResourceInner::TransferEncoded( buffer ) )
    }

    #[inline]
    pub fn from_future_encoded( fut: SendBoxFuture<TransferEncodedFileBuffer, Error> ) -> Self {
        Self::new_inner( ResourceInner::EncodingBuffer( fut ) )
    }

    pub fn state(&self) -> ResourceState {
        self.read_inner()
            .map(|inner| inner.state())
            .unwrap_or(ResourceState::HadError)
    }

    pub fn set_preferred_encoding( &mut self, tenc: TransferEncoding ) {
        self.preferred_encoding = Some( tenc )
    }

    pub fn get_preffered_encoding( &self ) -> Option<&TransferEncoding> {
        self.preferred_encoding.as_ref()
    }

    fn new_inner( r: ResourceInner ) -> Self {
        Resource {
            inner: Arc::new( RwLock::new( r ) ),
            preferred_encoding: None
        }
    }

    fn read_inner( &self ) -> Result<RwLockReadGuard<ResourceInner>> {
        match self.inner.read() {
            Ok( guard ) => Ok( guard ),
            Err( .. ) => bail!( "[BUG] lock was poisoned" )
        }
    }

    fn write_inner( &self ) -> Result<RwLockWriteGuard<ResourceInner>> {
        match self.inner.write() {
            Ok( guard ) => Ok( guard ),
            Err( .. ) => bail!( "[BUG] lock was poisoned" )
        }
    }

    pub fn get_if_encoded( &self ) -> Result<Option<Guard>> {
        use self::ResourceInner::*;
        let inner = self.read_inner()?;
        let ptr = match *inner {
            TransferEncoded( ref encoded )  => Some( encoded as *const TransferEncodedFileBuffer ),
            _ => None
        };

        Ok( ptr.map( |ptr |Guard {
            guard: inner,
            inner_ref: ptr,
            _marker: PhantomData
        } ) )
    }

    pub fn as_future<'a, C>( &'a mut self, ctx: &'a C ) -> ResourceFutureRef<'a, C> {
        ResourceFutureRef {
            resource_ref: self,
            ctx_ref: ctx
        }
    }

    pub fn poll_encoding_completion<C>( &mut self, ctx: &C ) -> Poll<(), Error>
        where C: BuilderContext
    {
        let mut inner = self.write_inner()?;
        let moved_out = mem::replace( &mut *inner, ResourceInner::Failed );
        let (move_back_in, state) =
            Resource::_poll_encoding_completion( moved_out, ctx, &self.preferred_encoding )?;
        mem::replace( &mut *inner, move_back_in );
        Ok( state )
    }

    fn _poll_encoding_completion<C>(
        resource: ResourceInner,
        ctx: &C,
        pref_enc: &Option<TransferEncoding>
    ) -> Result<(ResourceInner, Async<()>)>
        where C: BuilderContext
    {
        use self::ResourceInner::*;
        let mut continue_with = resource;
        // NOTE(why the loop):
        // we only return if we polled on a contained future and it resulted in
        // `Async::NotReady` or if we return `Async::Ready`. If we would not do
        // so the Spawn(/Run?/Task?) might think we are waiting for something _external_
        // and **park** the task e.g. by masking it as not ready in tokio or calling
        // `thread::park()` in context of `Future::wait`.
        //
        // Alternatively we also could call `task::current().notify()` in all
        // cases where we would return a `NotReady` from our side (e.g.
        // when we got a ready from file loading and advance the to `Loaded` )
        // but using a loop here should be better.
        loop {
            continue_with = match continue_with {
                Spec(spec) => {
                    let ResourceSpec { path, use_mime, use_name } = spec;
                    //we require a name
                    let name =
                        if let Some(name) = use_name {
                            name
                        } else {
                            let name = path
                                .file_name()
                                .ok_or_else(|| -> Error {
                                    ErrorKind::PathToFileWithoutFileName(path.to_owned()).into()
                                })?
                                .to_string_lossy()
                                .into_owned();
                            name
                        };
                    LoadingBuffer(
                        ctx.execute(
                            ctx.load_file( Cow::Owned( path ) )
                                .and_then(move |bytes| {

                                    //we require a mime/content-type
                                    let mime = detect_mime(&bytes, use_mime)?;

                                    //use now as read date
                                    let meta = FileMeta {
                                        file_name: Some(name),
                                        read_date: Some(now()),
                                        ..Default::default()
                                    };
                                    Ok(FileBuffer::new_with_file_meta(mime, bytes, meta))
                                })
                        )
                    )
                },

                LoadingBuffer(mut fut) => {
                    match fut.poll()? {
                        Async::Ready( buf )=> Loaded( buf ),
                        Async::NotReady => {
                            return Ok( ( LoadingBuffer(fut), Async::NotReady ) )
                        }
                    }
                },

                Loaded(buf) => {
                    let pe = pref_enc.clone();
                    EncodingBuffer( ctx.execute_fn(move || {
                        TransferEncodedFileBuffer::encode_buffer(buf, pe.as_ref())
                    } ) )
                },

                EncodingBuffer(mut fut) => {
                    match fut.poll()? {
                        Async::Ready( buf )=> TransferEncoded( buf ),
                        Async::NotReady => {
                            return Ok( ( EncodingBuffer(fut), Async::NotReady ) )
                        }
                    }
                },

                ec @ TransferEncoded(..) => {
                    return Ok( ( ec , Async::Ready( () ) ) )
                },

                Failed => {
                    bail!( "failed already in previous poll" );
                }
            }
        }
    }


    /// mainly for testing
    pub fn empty_text() -> Self {
        //OPTIMIZE use const MediaType once aviable
        let text_plain = MediaType::new("text","plain").unwrap();
        Resource::from_buffer( FileBuffer::new( text_plain, Vec::new() ) )
    }

}


impl<'a, C: 'a> Future for ResourceFutureRef<'a, C>
    where C: BuilderContext
{
    type Item = ();
    type Error = Error;

    fn poll( &mut self ) -> Poll<Self::Item, Self::Error> {
        self.resource_ref.poll_encoding_completion( self.ctx_ref )
    }
}

fn detect_mime<B: AsRef<[u8]>>(buffer: B, use_mime: Option<MediaType>) -> Result<MediaType> {
    Ok(if let Some(mime) = use_mime {
        mime
    } else {
        //FIXME tree_magic is far from optimal
        let media_type = tree_magic::from_u8(buffer.as_ref());
        let media_type: MediaType = MediaType::parse(media_type)
            .chain_err( || "[BUG] invalid media_type by tree_magic" )?;
        if media_type.type_() == TEXT {
            bail!("auto-detecting charset is currently not supported");
        }
        if media_type.type_() == APPLICATION
            && media_type.subtype() == OCTET_STREAM {
            bail!("auto-detection failed got application/octet-stream")
        }
        media_type
    })
}



impl fmt::Debug for ResourceInner {
    fn fmt( &self, fter: &mut fmt::Formatter ) -> fmt::Result {
        use self::ResourceInner::*;
        match *self {
            Spec( ref spec ) => <ResourceSpec as fmt::Debug>::fmt( spec, fter ),
            LoadingBuffer( .. ) => write!( fter, "LoadingBuffer( future )" ),
            Loaded( ref buf ) => <FileBuffer as fmt::Debug>::fmt( buf, fter ),
            EncodingBuffer( .. ) => write!( fter, "EncodingBuffer( future )" ),
            TransferEncoded( ref buf ) => <TransferEncodedFileBuffer as fmt::Debug>::fmt( buf, fter ),
            Failed => write!( fter, "Failed" )
        }
    }
}


impl<'a> Deref for Guard<'a> {
    type Target = TransferEncodedFileBuffer;

    fn deref( &self ) -> &TransferEncodedFileBuffer {
        //SAFE: the lifetime of the value behind the inner_ref pointer is bound
        // to the lifetime of the RwLock and therefore lives longer as
        // the Guard which is also part of this struct and therefore
        // has to life at last as long as the struct
        unsafe { &*self.inner_ref }
    }
}

impl BodyBuffer for Resource {
    fn with_slice<FN, R>(&self, func: FN) -> Result<R>
        where FN: FnOnce(&[u8]) -> Result<R>
    {
        if let Some( guard ) = self.get_if_encoded()?{
            func(&*guard)
        } else {
            bail!("buffer has not been encoded yet");
        }

    }
}


#[cfg(test)]
mod test {
    use std::fmt::Debug;
    use futures::Future;
    use futures::future::Either;

    use futures_cpupool::{CpuPool, Builder};

    use super::*;

    use context::CompositeBuilderContext;
    use default_impl::VFSFileLoader;

    use utils::timeout;

    type SimpleContext = CompositeBuilderContext<VFSFileLoader, CpuPool>;

    fn resolve_resource<C: BuilderContext+Debug>( resource: &mut Resource, ctx: &C ) {
        let res = resource
            .as_future( ctx )
            .select2( timeout( 1, 0 ) )
            .wait()
            .unwrap();

        match res {
            Either::A( .. ) => { },
            Either::B( .. ) => {
                panic!( "timeout! resource as future did never resolve to either Item/Error" )
            }
        }
    }

    fn cpupool() -> CpuPool {
        Builder::new().create()
    }

    #[test]
    fn load_test() {
        let mut fload = VFSFileLoader::new();
        fload.register_file( "/test/me.yes", b"abc def!".to_vec() ).unwrap();
        let ctx = SimpleContext::new( fload, cpupool() );

        let spec = ResourceSpec {
            path: "/test/me.yes".into(),
            use_name: None,
            use_mime: Some( MediaType::parse("text/plain;charset=us-ascii").unwrap() )
        };

        let mut resource = Resource::from_spec( spec );

        assert_eq!( false, resource.get_if_encoded().unwrap().is_some() );

        resolve_resource( &mut resource, &ctx );

        let res = resource.get_if_encoded().unwrap().unwrap();
        let enc_buf: &TransferEncodedFileBuffer = &*res;
        let data: &[u8] = &*enc_buf;
        
        assert_eq!( b"abc def!", data );
    }


    #[test]
    fn load_test_utf8() {
        let mut fload = VFSFileLoader::new();
        fload.register_file( "/test/me.yes", "Öse".as_bytes().to_vec() ).unwrap();
        let ctx = SimpleContext::new( fload, cpupool() );

        let spec = ResourceSpec {
            path: "/test/me.yes".into(),
            use_name: None,
            use_mime: Some( MediaType::parse("text/plain;charset=utf8").unwrap() )
        };

        let mut resource = Resource::from_spec( spec );

        assert_eq!( false, resource.get_if_encoded().unwrap().is_some() );

        resolve_resource( &mut resource, &ctx );

        let res = resource.get_if_encoded().unwrap().unwrap();
        let enc_buf: &TransferEncodedFileBuffer = &*res;
        let data: &[u8] = &*enc_buf;

        assert_eq!( b"=C3=96se", data );
    }


    #[test]
    fn from_text_works() {
        let mut resource = Resource::from_text( "orange juice".into() );
        resolve_resource( &mut resource, &SimpleContext::new( VFSFileLoader::new(), cpupool() ) );
        let res = resource.get_if_encoded().unwrap().unwrap();
        let data: &[u8] = &*res;
        assert_eq!( b"orange juice", data );
    }




}