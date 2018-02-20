use std::marker::PhantomData;
use std::fmt;
use std::sync::{Arc, RwLock, RwLockWriteGuard, RwLockReadGuard};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::ops::Deref;
use std::mem;


use futures::{  Future, Poll, Async };
use futures::task;

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

#[derive( Debug, Clone )]
pub struct Resource {
    //TODO change to Arc<Inner> with Inner { source: Source, state: RwLock<State> }
    // using inner.source(), inner.state_mut()-> Lock, inner.state() -> Lock
    inner: Arc<ResourceInner>,
}

#[repr(usize)]
#[derive(Debug, Eq, PartialEq, Copy, Clone, Hash)]
pub enum ResourceStateInfo {
    /// The resource is not loaded, but can be loaded at this state
    /// can only be reached if the Resource has a source
    NotLoaded,
    /// The resource is in a state between `NotLoaded` and `Loaded`
    Loading,
    /// The resource is completely loaded (and content transfer encoded) and
    /// can be used, as long as it is not unloaded.
    Loaded,
    /// Loading the resource failed due to a error, if the resource has a `Source`
    /// `Resource.try_unload` can be used to transform it back into the `NotLoaded` state.
    Failed,

    /// Was in the process of beeing loaded but loading was canceld,
    /// potentially has already loaded the data but not transfer encoded
    /// it.
    Canceled
}


//FIXME use AtomicU8 once stable: https://github.com/rust-lang/rust/issues/32976
#[derive(Debug)]
struct AtomicStateInfo(AtomicUsize);

impl AtomicStateInfo {
    fn new(state: ResourceStateInfo) -> Self {
        AtomicStateInfo(AtomicUsize::new(state as usize))
    }

    fn set(&self, state: ResourceStateInfo) {
        self.0.store(state as usize, Ordering::Release)
    }

    /// Use this to check if polling was canceld and continue if needed
    ///
    /// If this returns `Canceled`, then it was canceld and not is `Loading` and
    /// the caller has to make sure to aktually do loading.
    ///
    /// Else the current state info is returned.
    fn try_continue_from_cancel(&self) -> ResourceStateInfo {
        let usize_state = self.0.compare_and_swap(
            // from:
            ResourceStateInfo::Canceled as usize,
            // to:
            ResourceStateInfo::Loading as usize,
            // I need Acquire failure ordering as I use the failure value,
            // which is what will happen if AcqRel is used
            Ordering::AcqRel
        );
        unsafe { mem::transmute::<usize, ResourceStateInfo>(usize_state) }
    }

    fn get(&self) -> ResourceStateInfo {
        let val = self.0.load(Ordering::Acquire);
        unsafe { mem::transmute::<usize, ResourceStateInfo>(val) }
    }
}


#[derive(Debug)]
struct ResourceInner {
    //CONSTRAINT: assert!(state.is_loaded() || source.is_some())
    state: RwLock<ResourceState>,
    source: Option<Source>,

    /// we need this for multiple reasons
    ///
    /// 1. Prevent starvation if someone thinks it's a good idea to run
    ///    `loop { resource.get_if_encoded() }`. Or _any_ method taking the lock (`state_info`, etc.)
    /// 2. allows a `ResourceLoadingFuture` which now some one else polls to _not_ need to
    ///    lock anything (which can slow down the poll which drives the future)
    /// 3. allows marking the canceled sub-state which can be combined with _any_ `ResourceState`
    ///    which is needed if multiple sources try to load a shared resource and the one which does
    ///    the polling cancels, else the other task would just continuously check if work is done
    state_info: AtomicStateInfo,

    /// prevents unloading while this resource is still in use
    /// originally I wanted to use a WriteLock downgrade but that
    /// does not work, due to the way it's implemented in parking_lot.
    ///
    /// (And not provided by std, also we can't "own" locks which is
    ///  another problem which can be worked around in parking_lot with
    ///  unsafe but not std)
    ///
    /// # Usage Semantics
    ///
    /// - This variable should only be increased/decresed by using `AntiUnloadLock`s
    /// - They must only be constructed while the `state.write()` lock is held
    ///     - else there could be a timing race between try_unload geting the lock and
    ///       new AntiUnloadLock's being created
    /// - Through they can be cloned/droped without the lock
    unload_prevention: AtomicUsize,
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

#[derive(Debug)]
pub struct ResourceLoadingFuture<C: BuilderContext> {
    inner: Arc<ResourceInner>,
    ctx: C,
    poll_state: PollState,
    anti_unload: Option<AntiUnloadGuard>
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
enum PollState {
    NotPolled,
    CanPoll,
    /// The load future refers to a state in a `Arc<...RwLock<...>>` other futures can refere to it
    /// _at the same time_ an theoretically they could poll in an interwinding manner. But this is
    /// a problem, futures only remember the last task and only nodify what they remember.
    ///
    /// E.g. if thread 1 (T1) & thread 2 (T2) both want to load the same resource (using wait)
    /// which means both will call poll on an inner future in ResourceState (synced by RwLock)
    /// the first (here T1) will get a `NotReady` and will "sleep", the future will remember to
    /// wake it if needed, the scond will also poll get also a `NotReady` and the future will now
    /// remember it's taks _instead of T1's taks_ when it's ready if will notify T2, but not T1 as
    /// it already forgot about it.
    SomeOneElsePolls,
    Done
}

#[derive(Debug)]
pub struct AntiUnloadGuard {
    handle: Arc<ResourceInner>,
}



impl Resource {

    fn _new(state: ResourceState, source: Option<Source>) -> Self {
        let state_info = state.state_info();
        debug_assert!(state_info != ResourceStateInfo::NotLoaded || source.is_some());
        Resource {
            inner: Arc::new(ResourceInner {
                source,
                state: RwLock::new(state),
                state_info: AtomicStateInfo::new(state_info),
                unload_prevention: AtomicUsize::new(0)
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
        self.inner.state_info()
    }

    pub fn is_loaded(&self) -> bool {
        self.inner.is_loaded()
    }

    pub fn get_if_encoded( &self ) -> Option<Guard> {
        self.inner.get_if_encoded()
    }

    pub fn create_loading_future<C>(&self, ctx: C) -> ResourceLoadingFuture<C>
        where C: BuilderContext
    {
        ResourceLoadingFuture::new(self.clone(), ctx)
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
        self.inner.try_unload()
    }

    /// true, if the `Resource` has a `Source` and therefore can be unloaded
    ///
    /// It is also true if the resource is corupted and therefore already not in
    /// a loaded state.
    pub fn can_be_unloaded(&self) -> bool {
        self.inner.can_be_unloaded()
    }
}

impl ResourceInner {

    fn is_loaded(&self) -> bool {
        self.state_info() == ResourceStateInfo::Loaded
    }

    fn can_be_unloaded(&self) -> bool {
        self.source().is_some()
    }

    fn state_info(&self) -> ResourceStateInfo {
        self.state_info.get()
    }

    fn set_state_info(&self, info: ResourceStateInfo) {
        self.state_info.set(info)
    }

    fn try_continue_from_cancel(&self) -> ResourceStateInfo {
        self.state_info.try_continue_from_cancel()
    }


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

    fn try_state_mut(&self) -> Option<RwLockWriteGuard<ResourceState>> {
        use std::sync::TryLockError::*;
        match self.state.try_write() {
            Ok(lock) => Some(lock),
            Err(Poisoned(plock)) => Some(plock.into_inner()),
            Err(WouldBlock) => None
        }
    }

    fn source(&self) -> Option<&Source> {
        self.source.as_ref()
    }

    fn try_unload(&self) -> Result<()> {
        use self::ResourceStateInfo::*;
        match self.state_info() {
            NotLoaded => Ok(()),
            //TODO typed error
            Loading => Err("resource is in use, can't unload it".into()),
            Loaded | Canceled | Failed => self._try_unload()
        }
    }

    fn _try_unload(&self) -> Result<()> {
        if self.source().is_some() {
            if 0 == self.unload_prevention.load(Ordering::Acquire) {
                if let Some(mut state) = self.try_state_mut() {
                    // there might have been a load/unload prevention before we got the lock
                    if 0 == self.unload_prevention.load(Ordering::Acquire) {
                        *state = ResourceState::NotLoaded;
                        return Ok(());
                    }
                }
            }
            //TODO typed error
            Err("can not unload source locked with AntiUnloadLock".into())
        } else {
            //TODO typed error
            Err("can not unload sourceless resource".into())
        }
    }


    fn get_if_encoded(&self) -> Option<Guard> {
        //this is a to not require a lock and prevent interfering with loading
        if self.is_loaded() {
            self._get_if_encoded()
        } else {
            None
        }
    }

    fn _get_if_encoded(&self) -> Option<Guard> {
        use self::ResourceState::*;

        let state_guard = self.state();
        let ptr = match *state_guard {
            TransferEncoded( ref encoded )  => Some( encoded as *const TransferEncodedFileBuffer ),
            _ => None
        };

        ptr.map(|ptr | Guard {
            guard: state_guard,
            state_ref: ptr,
            _marker: PhantomData
        })
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

    fn poll_encoding_completion<C>(self, source: &Option<Source>, ctx: &C)
                                   -> Result<(ResourceState, Async<()>)>
        where C: BuilderContext
    {
        use self::ResourceState::*;
        let mut continue_with = self;
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
                    let source: &Source = source.as_ref()
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
        if let Some( guard ) = self.get_if_encoded() {
            func(&*guard)
        } else {
            bail!("buffer has not been encoded yet");
        }

    }
}

impl<C> ResourceLoadingFuture<C>
    where C: BuilderContext
{
    fn new(resource: Resource, ctx: C) -> Self {
        ResourceLoadingFuture {
            inner: resource.inner, ctx,
            poll_state: PollState::NotPolled,
            anti_unload: None
        }
    }

    fn _poll_inner(&mut self)
        -> Poll<AntiUnloadGuard, Error>
    {
        // we ill will use state_mut() which get's a RwLock and _blocks_ which is bad for a poll
        // _but_ this should not be a problem, the only think in the lock-section is to
        //
        // 1. which state (one enum match)
        // 2. poll inner future
        // 3. set new state
        //
        // And in case of the first poll a new AntiUnloadGuard (~ 2 Atomic adds, no heap alloc).
        // So this should be fine.
        //
        // The only longer locking operation are longer read lock when using a resource,
        // BUT in that case it's already loaded, so we do not need a write lock at all.
        let helper = self.mut_helper();
        match helper.inner.try_state_mut() {
            // can happen if it was already loaded but all AntiUnloadGuard's had been dropped
            None => helper.poll_inner_no_lock(),
            Some(guard) => helper._poll_inner_with_lock(guard)
        }
    }

    fn mut_helper(&mut self) -> MutHelper<C> {
        MutHelper {
            inner: &self.inner,
            poll_state: &mut self.poll_state,
            anti_unload: &mut self.anti_unload,
            ctx: &self.ctx
        }
    }

}

// work around for borrow checker limitation
// (I basically split a self borrow into a mutable part {poll_state, anti_unload} and a immutable
//  part {inner, ctx})
struct MutHelper<'a, C: BuilderContext> {
    inner: &'a ResourceInner,
    poll_state: &'a mut PollState,
    anti_unload: &'a mut Option<AntiUnloadGuard>,
    ctx: &'a C
}

impl<'a, C: 'a> MutHelper<'a, C>
    where C: BuilderContext
{
    fn poll_inner_no_lock(self) -> Poll<AntiUnloadGuard, Error> {
        if self.inner.is_loaded() {
            Ok(self.make_done())
        } else {
            // Should not happen. All info methods use the state_info atomic.
            // But if it does, we know it's not long term (as it is not loaded)
            // and just try again next tick
            task::current().notify();
            Ok(Async::NotReady)
        }
    }

    fn _poll_inner_with_lock(self, mut state_guard: RwLockWriteGuard<ResourceState>)
                             -> Poll<AntiUnloadGuard, Error>
    {
        // do our own kind of poisoning 1. because of borrowing 2. for unwind safety/no lock poison
        let moved_out = mem::replace(&mut *state_guard, ResourceState::Failed);

        let (move_back_in, async_state) =
            moved_out.poll_encoding_completion(&self.inner.source, self.ctx)?;

        //Note: we still have the write lock and while people will read from state_info
        // even if we have the lock they won't write to it with out the lock (except if
        // the state_info is `canceld` but then we would not be here calling poll, as this
        // state is only reached if the future is dropped before polling to completion)
        self.inner.set_state_info(move_back_in.state_info());

        mem::replace(&mut *state_guard, move_back_in);

        Ok(match async_state {
            Async::NotReady => Async::NotReady,
            Async::Ready(()) => {
                self.make_done()
            }
        })
    }


    fn make_done(self) -> Async<AntiUnloadGuard> {
        *self.poll_state = PollState::Done;
        let anti = self.anti_unload.take()
            .expect("[BUG] anti is always set when polling started, only removed once it ends");
        Async::Ready(anti)
    }
}


impl<C> Drop for ResourceLoadingFuture<C>
    where C: BuilderContext
{
    fn drop(&mut self) {
        // we don't really need this but if someone wrongly uses
        // drop_in_place or similar this can safe us a lot of
        // headache as it prevents polling to continue
        if self.poll_state != PollState::Done {
            self.poll_state = PollState::Done;
            self.inner.set_state_info(ResourceStateInfo::Canceled);
        }
    }
}

impl<C> Future for ResourceLoadingFuture<C>
    where C: BuilderContext
{
    type Item = AntiUnloadGuard;
    type Error = Error;


    fn poll( &mut self ) -> Poll<Self::Item, Self::Error> {

        use self::PollState::*;
        match self.poll_state {
            NotPolled => {
                // if is_inital == true, then we need to poll, else some one else does it for us
                // (or it's already loaded)
                let (anti, is_initial) = AntiUnloadGuard::new(&self.inner);
                self.anti_unload = Some(anti);
                if is_initial {
                    self.poll_state = CanPoll;
                } else {
                    self.poll_state = SomeOneElsePolls;
                }
                self._poll_inner()
            },
            CanPoll => {
                self._poll_inner()
            },
            SomeOneElsePolls => {
                let state = self.inner.try_continue_from_cancel();
                match state {
                    ResourceStateInfo::Loaded => Ok(self.mut_helper().make_done()),
                    //TODO typed error
                    ResourceStateInfo::Failed => Err("resource loading failed".into()),
                    ResourceStateInfo::Loading | ResourceStateInfo::NotLoaded => {
                        // this will prevent a sleep forever scenario but it also means that the
                        // Executor will poll this future one every tick, not optimal but acceptable
                        // (to change this every `InnerResource` would need a queue to enqueue all not
                        //  polling futures. Given how Resource is meant to be used this might not be
                        //  worth the extra effort)
                        //FEAT: bench speed+size if a extra task queue would be worth it
                        task::current().notify();
                        Ok(Async::NotReady)
                    }
                    ResourceStateInfo::Canceled => {
                        // now we are the one to drive the future to completion
                        self.poll_state = CanPoll;
                        self._poll_inner()
                    }
                }
            },
            //TODO typed error
            Done => panic!("[BUG] called poll after future was resolved, use fuse if you have to")
        }
    }
}



/// Keep alive to prevent a resource from beeing unloaded.
///
impl AntiUnloadGuard {

    /// Creates a new `AntiUnloadGuard` returning it and a bool indicating if it's the first guard.
    ///
    /// who ever creates a AntiUnloadGuard which is the first is responsible for loading the
    /// resource it's belongs to (no one else will do so, at last not until the given guard
    /// and all others created after it are dropped).
    fn new(resource: &Arc<ResourceInner>) -> (Self, bool) {
        let handle = resource.clone();
        let prev = handle.unload_prevention.fetch_add(1, Ordering::Release);
        (AntiUnloadGuard { handle }, prev == 0)
    }

    /// return the loaded resource
    //NOTE: this would panic if called befor the resource was loaded, but any
    // `AntiUnloadGuard` will only exposed to pub once a resource was loaded succsefully.
    //(before it will lay dormant in the `ResourceLoadFuture` where we need it to prevent
    // interchanging double loads)
    pub fn access(&self) -> Guard {
        self.handle.get_if_encoded()
            .expect("[BUG] should only be accessible when loaded succesfull")
    }

}

impl Clone for AntiUnloadGuard {

    fn clone(&self) -> Self {
        let handle = self.handle.clone();
        handle.unload_prevention.fetch_add(1, Ordering::Release);
        AntiUnloadGuard { handle }
    }
}

impl Drop for AntiUnloadGuard {

    fn drop(&mut self) {
        // decrease the unload_prevention count, as we added one when construction and no one else
        // does access the variable this won't underflow
        self.handle.unload_prevention.fetch_sub(1, Ordering::Release);
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
            .create_loading_future(ctx.clone())
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

        assert_eq!( false, resource.get_if_encoded().is_some() );

        resolve_resource( &mut resource, &ctx );

        let res = resource.get_if_encoded().unwrap();
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