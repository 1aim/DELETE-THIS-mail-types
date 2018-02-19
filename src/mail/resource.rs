use std::marker::PhantomData;
use std::fmt;
use std::sync::{ Arc, RwLock, RwLockWriteGuard, RwLockReadGuard };
use std::ops::Deref;
use std::mem;

use futures::{  Future, Poll, Async };

use core::error::{Error, Result};
use core::codec::BodyBuffer;

use utils::SendBoxFuture;
use file_buffer::{FileBuffer, TransferEncodedFileBuffer};
use super::context::{BuilderContext, Source};


//TODO as resources now can be unloaded I need to have some form of handle which
//     assures that between loading and using a resource it's not unloaded
//TODO as resources can be shared between threads it's possible to load it in two places at once
//     currently this means it's possible that two threads whill interchangably call load
//     which might not be the best idea. Consider taking advantage of the Lock and moving
//     it into the Future of as_future or so

#[derive(Debug)]
pub struct ResourceFutureRef<'a, C: 'a> {
    resource_ref: &'a mut Resource,
    ctx_ref: &'a C
}

#[derive( Debug, Clone )]
pub struct Resource {
    //TODO change to Arc<Inner> with Inner { source: Source, state: RwLock<State> }
    // using inner.source(), inner.state_mut()-> Lock, inner.state() -> Lock
    inner: Arc<ResourceInner>,
}

#[derive(Debug, Eq, PartialEq, Copy, Clone, Hash)]
pub enum ResourceStateInfo {
    /// The resource is not loaded, but can be loaded at this state
    /// can only be reached if the Resource has a source
    NotLoaded,
    //TODO loading does not differ btween from FileBuffer but not actually in the process of
    //     beeing loaded and actually loaded
    /// The resource is in a state between `NotLoaded` and `Loaded`
    Loading,
    /// The resource is completely loaded (and content transfer encoded) and
    /// can be used, as long as it is not unloaded.
    Loaded,
    /// Loading the resource failed due to a error, if the resource has a `Source`
    /// `Resource.try_unload` can be used to transform it back into the `NotLoaded` state.
    Failed
}


#[derive(Debug)]
struct ResourceInner {
    //CONSTRAINT: assert!(state.is_loaded() || source.is_some())
    state: RwLock<ResourceState>,
    source: Option<Source>,
}

enum ResourceState {
    NotLoaded,
    LoadingBuffer( SendBoxFuture<FileBuffer, Error> ),
    Loaded( FileBuffer ),
    EncodingBuffer( SendBoxFuture<TransferEncodedFileBuffer, Error> ),
    TransferEncoded( TransferEncodedFileBuffer ),
    Failed
}

pub struct Guard<'lock> {
    //NOTE: this is NOT dead_code (field never used),
    // just unused through it still _drops_ and has a _side effect_
    // on drop (which is what rustc's lint does not "know")
    #[allow(dead_code)]
    guard: RwLockReadGuard<'lock, ResourceState>,
    state_ref: *const TransferEncodedFileBuffer,
    // given that we neither own a value we point to (DropCheck) nor
    // have a unused type parameter nor lifetime this is probably not
    // needed, still it's better to be safe and have this zero-runtime-overhead
    // marker
    _marker: PhantomData<&'lock TransferEncodedFileBuffer>
}



impl Resource {

    fn _new(state: ResourceState, source: Option<Source>) -> Self {
        debug_assert!(state.state_info() != ResourceStateInfo::NotLoaded || source.is_some());
        Resource {
            inner: Arc::new(ResourceInner {
                source,
                state: RwLock::new(state)
            }),
        }
    }

    pub fn new(source: Source) -> Self {
        Resource::_new(ResourceState::NotLoaded, Some(source))
    }

    /// This constructor allow crating a Resource from a FileBuffer without providing a source IRI
    ///
    /// This is useful in combination with e.g. "on-the-fly" generated resources. A Resource
    /// created this way can not be unloaded, as such this preferably should only be used with
    /// "one-use" resources which do not need to be cached.
    pub fn sourceless_from_buffer( buffer: FileBuffer ) -> Self {
        Self::_new( ResourceState::Loaded( buffer ), None )
    }

    /// This constructor allow crating a Resource from a Future resolving to a FileBuffer
    /// without providing a source IRI
    ///
    /// This is useful in combination with e.g. "on-the-fly" generated resources. A Resource
    /// created this way can not be unloaded, as such this preferably should only be used with
    /// "one-use" resources which do not need to be cached.
    pub fn sourceless_from_future( fut: SendBoxFuture<FileBuffer, Error> ) -> Self {
        Self::_new( ResourceState::LoadingBuffer( fut ), None )
    }


    pub fn state_info(&self) -> ResourceStateInfo {
        self.inner.state().state_info()
    }


    pub fn get_if_encoded( &self ) -> Result<Option<Guard>> {
        use self::ResourceState::*;
        let state_guard = self.inner.state();
        let ptr = match *state_guard {
            TransferEncoded( ref encoded )  => Some( encoded as *const TransferEncodedFileBuffer ),
            _ => None
        };

        Ok( ptr.map( |ptr |Guard {
            guard: state_guard,
            state_ref: ptr,
            _marker: PhantomData
        } ) )
    }

    pub fn as_future<'a, C>(&'a mut self, ctx: &'a C) -> ResourceFutureRef<'a, C> {
        ResourceFutureRef {
            resource_ref: self,
            ctx_ref: ctx,
        }
    }

    pub fn source(&self) -> Option<&Source> {
        self.inner.source()
    }

    /// Tries to transform the resource into a state where it is not loaded.
    ///
    /// This requires the resource to have a source.
    ///
    /// This was designed this way as `try_unload` is mainly meant to bee a function
    /// to free some memory in a cache or `Resources` NOT as a tool to enforce a resource
    /// is not loade. Due to the shared nature of resources it is possible that e.g.
    /// between a call to `try_unload` and a imediatly following call to `state_info` the
    /// resource was already loaded again (or is in the process of). As such a call to
    /// `try_unload` should **only be done once in a while and never in any form of loop**
    /// or else it is possible that one thread load the resource to make sure its aviable
    /// when it needs it while the other thread continuously unloads it.
    ///
    /// # Error
    /// an error occurs if the resource can not be changed into a
    /// state where it is not loaded. This can only happen if
    /// `try_unload` is used on a `Resource` which has no `Source`.
    ///
    pub fn try_unload(&self) -> Result<()> {
        if self.source().is_some() {
            *self.inner.state_mut() = ResourceState::NotLoaded;
            Ok(())
        } else {
            //TODO typed error
            return Err("can not unload sourceless resource".into())
        }
    }

    /// true, if the `Resource` has a `Source` and therefore can be unloaded
    ///
    /// It is also true if the resource is corupted and therefore already not in
    /// a loaded state.
    pub fn can_be_unloaded(&self) -> bool {
        self.inner.source().is_some()
    }

    pub fn poll_encoding_completion<C>( &mut self, ctx: &C ) -> Poll<(), Error>
        where C: BuilderContext
    {
        let mut state = self.inner.state_mut();
        //TODO this already works like poisoning so we don't really need the lock poisoning
        //  Solutions:
        //     a) use parking lot it's faster and does not implement lock poisoning
        //     b) have a catch + resume unwind handle which releses the lock before resuming
        //     c) [for now] if write_inner stumbles across poison it will
        let moved_out = mem::replace(&mut *state, ResourceState::Failed );
        let (move_back_in, async_state) = self._poll_encoding_completion(moved_out, ctx)?;
        mem::replace( &mut *state, move_back_in );
        Ok( async_state )
    }

    fn _poll_encoding_completion<C>(&self, resource: ResourceState, ctx: &C)
        -> Result<(ResourceState, Async<()>)>
        where C: BuilderContext
    {
        use self::ResourceState::*;
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
                NotLoaded => {
                    let source: &Source = self.source()
                        .ok_or_else(|| -> Error {
                            //TODO typed error
                            "[BUG] illegal state no source and not loaded".into()
                        })?;

                    LoadingBuffer(
                        ctx.load_resource(source)
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
                    EncodingBuffer( ctx.offload_fn(move || {
                        TransferEncodedFileBuffer::encode_buffer(buf, None)
                    }))
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
                    //TODO typed error
                    bail!( "failed already in previous poll" );
                }
            }
        }
    }
}

impl ResourceInner {

    fn state( &self ) -> RwLockReadGuard<ResourceState> {
        match self.state.read() {
            Ok( guard ) => guard,
            Err( poisoned ) => {
                // we already have our own form of poisoning with mem-replacing state with Failed
                // during the any mutating operation which can panic (which currently only is poll)
                let guard = poisoned.into_inner();
                guard
            }
        }
    }

    /// # Unwindsafty
    ///
    /// This method accesses the inner lock regardless of
    /// poisoning the reason why this is fine is that all
    /// operations which modify the guarded value _and_ can
    /// panic do have their own form of poisoning. Currently
    /// this is just `poll_encoding_completion` which does
    /// modify the inner `stat` and uses `Failed` as a form
    /// of poison state. **Any usecase which can panic needs
    /// to make sure it's unwind safe _without lock poisoning_**
    fn state_mut( &self ) -> RwLockWriteGuard<ResourceState> {
        match self.state.write() {
            Ok( guard ) => guard,
            Err( poisoned ) => {
                // we already have our own form of poisoning with mem-replacing state with Failed
                // during the any mutating operation which can panic (which currently only is poll)
                let guard = poisoned.into_inner();
                guard
            }
        }
    }

    fn source(&self) -> Option<&Source> {
        self.source.as_ref()
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


impl fmt::Debug for ResourceState {
    fn fmt( &self, fter: &mut fmt::Formatter ) -> fmt::Result {
        use self::ResourceState::*;
        match *self {
            NotLoaded => write!(fter, "NotLoaded"),
            LoadingBuffer( .. ) => write!( fter, "LoadingBuffer( <future> )" ),
            Loaded( ref buf ) => <FileBuffer as fmt::Debug>::fmt( buf, fter ),
            EncodingBuffer( .. ) => write!( fter, "EncodingBuffer( <future> )" ),
            TransferEncoded( ref buf ) => <TransferEncodedFileBuffer as fmt::Debug>::fmt( buf, fter ),
            Failed => write!( fter, "Failed" )
        }
    }
}

impl ResourceState {

    fn state_info(&self) -> ResourceStateInfo {
        use self::ResourceState::*;
        match *self {
            NotLoaded => ResourceStateInfo::NotLoaded,
            TransferEncoded(..) => ResourceStateInfo::Loaded,
            Failed => ResourceStateInfo::Failed,
            _ => ResourceStateInfo::Loading
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
        unsafe { &*self.state_ref }
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
    use std::path::{Path, PathBuf};
    use std::fmt::Debug;

    use futures::Future;
    use futures::future::Either;

    use ::{IRI, MediaType};

    use super::*;

    use default_impl::test_context;

    use utils::timeout;


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

    fn load_file<P: AsRef<Path>>(path: P) -> Vec<u8> {
        use std::io::Read;
        use std::fs::File;
        let mut file = File::open(path).unwrap();
        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer).unwrap();
        buffer
    }

    #[test]
    fn load_test() {
        let ctx = test_context();
        let iri = IRI::from_parts("path", "./test_resources/text.txt").unwrap();
        let path = PathBuf::from(iri.tail());
        let source = Source {
            iri,
            use_media_type: Some(MediaType::parse("text/plain; charset=utf-8").unwrap()),
            use_name: None
        };

        let mut resource = Resource::new( source );

        assert_eq!( false, resource.get_if_encoded().unwrap().is_some() );

        resolve_resource( &mut resource, &ctx );

        let res = resource.get_if_encoded().unwrap().unwrap();
        let enc_buf: &TransferEncodedFileBuffer = &*res;
        let expected = load_file(path);
        assert_eq!( enc_buf.as_slice() , expected.as_slice());
    }


//    #[test]
//    fn load_test_utf8() {
//        let mut fload = VFSFileLoader::new();
//        fload.register_file( "/test/me.yes", "Öse".as_bytes().to_vec() ).unwrap();
//        let ctx = SimpleContext::new(fload, simple_cpu_pool() );
//
//        let spec = Source {
//            iri: "/test/me.yes".into(),
//            use_media_type: MediaType::parse("text/plain;charset=utf8").unwrap(),
//            use_name: None,
//        };
//
//        let mut resource = Resource::from_spec( spec );
//
//        assert_eq!( false, resource.get_if_encoded().unwrap().is_some() );
//
//        resolve_resource( &mut resource, &ctx );
//
//        let res = resource.get_if_encoded().unwrap().unwrap();
//        let enc_buf: &TransferEncodedFileBuffer = &*res;
//        let data: &[u8] = &*enc_buf;
//
//        assert_eq!( b"=C3=96se", data );
//    }



}