use std::marker::PhantomData;
use std::fmt;
use std::sync::{Arc, RwLock, RwLockWriteGuard, RwLockReadGuard};
use std::result::{Result as StdResult};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::ops::Deref;
use std::mem;


use futures::{  Future, Poll, Async };
use futures::task;
use failure::Backtrace;

use common::error::{EncodingError, EncodingErrorKind};
use common::encoder::BodyBuffer;

use ::error::{ResourceError, ResourceLoadingError, ResourceLoadingErrorKind}
use ::utils::SendBoxFuture;
use ::file_buffer::{FileBuffer, TransferEncodedFileBuffer};
use super::context::{BuilderContext, Source};

/// A Resource represent something which can be a body (part) of a Mail.
///
/// # Creation
///
/// There are two ways to create a `Resource`.
///
/// One is from a `Source`, which is a description about where to
/// get the resource from (IRI) and how to treat it (Media Type/Name).
/// Resources created this way have the benefit that they can be
/// unloaded to free memory if they are temporary not needed.
///
/// The other is  way is to create a Resource without an `Source`
/// from a `FileBuffer` or and future resolving to an `FileBuffer`.
/// As it can not be reloaded it can not be unloaded (throug you
/// still can drop it)
///
/// # Sharing
///
/// Resources are meant to be easy to share, as such there inner state
/// is in an `Arc` so cloning them is sheap. Not only this if a shared
/// resource is loaded and/or transfer encoded all other users of the
/// shared resource do profit from it and do not have to do it again.
///
/// E.g. if you want to embed a logo in all HTML mails you send it only
/// has to be loaded and transfer encoded once.
///
/// Additionally it is possible to unload resources which have a `Source`.
/// Doing so which will free the loaded and encoded data contained, but
/// you still can freely clone it and pass it around, reloading the content
/// once you need it. Allowing you to directly use Resources in both some
/// template describing data structur and a LRU cache, without woring, that
/// one will prevent the other from doing it's job (i.e. the LRU cache should
/// use `try_unload` when "dropping" unused resource).
///
/// # Loading / Using
///
/// When a Resource is created from a `Source` it does not yet contain any data,
/// if it is created without a `Source` it already contains data, but it might
/// not yet be transfer encoded.
///
/// To access the data of an resource (and load it before hand) use the
/// `create_loading_future` to create a `ResourceLoadingFuture` which will
/// resolve into `ResourceAccessGuard` once the future is loaded. The `ResourceAccessGuard`
/// serves two purposes:
///
/// 1. it prevents Resources from being unloaded while you still intend to use it
///    (e.g. a Resource from a LRU cache)
/// 2. it gives you access to the underlying `TransferEncodedFileBuffer`
///
/// Note that even without a `ResourceAccessGuard` you can get access to the underlying
/// buffer if a resource is loaded and you use `get_if_encoded`.
///
/// As a `Resource` can be shared it is possible that there are multiple
/// `ResourceLoadingFuture`'s for the same resource. The Future+Resource do
/// make sure that this process is cordinated and only one future actually
/// drives the completation while the others wait for it to be done (or to
/// be canceled so that they can pick up the polling). This is necessary due
/// to the way the future libary parks and notifies tasks.
///
/// # Example
///
//TODO
// ```
// //1. context
// //2. mk resource clone it
// //3. load it
// //4. use both instances
// ```
///
#[derive( Debug, Clone )]
pub struct Resource {
    inner: Arc<ResourceInner>,
}

/// A enum representing the inner state of the resource, i.e. if it is loading, loaded, etc.
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

    /// Was in the process of being loaded but loading was canceled,
    /// potentially has already loaded the data but not transfer encoded it.
    Canceled
}

/// A wrapper alowing us to have a atomic `ResourceStateInfo` by converting
/// it from/to a AtomicUsize
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

/// The inner Resource normally accessed through an `Arc`
#[derive(Debug)]
struct ResourceInner {
    //CONSTRAINT: assert!(state.is_loaded() || source.is_some())
    //CONSTRAINT: the future in ResourceState can only be accessed in exclusive lock mode
    //            using it in read mead would require it to be send, which it isn't
    state: RwLock<ResourceState>,
    source: Option<Source>,

    /// we need this for multiple reasons
    ///
    /// 1. Prevent starvation if someone thinks it's a good idea to run
    ///    `loop { resource.get_if_encoded() }`. Or _any_ method taking the lock (`state_info`, etc.)
    /// 2. allows a `ResourceLoadingFuture` which now some one else polls to _not_ need to
    ///    lock anything (which can hinder the poll which drives the future)
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
    /// - This variable should only be increased/decreased by using `ResourceAccessGuard`
    /// - They should only be constructed with `new` in `ResourceLoadingFutur::poll` as
    ///   any task creating a `ResourceAccessGuard` and gets returned that it's a initial one
    ///   _has to_ load the future (having only on place to inc. it makes thinks a lot easier)
    /// - If we want to add a `get_anti_unload_guard_if_loaded` method or similar we have
    ///   to make sure the syn. is fine i.e. 1. only create if we are in a `Loaded` state
    ///   onve created check that we _still_ are in the loaded state if not we need to dec.
    ///   the count and not return the new `AnitUnloadGuard` also if we do so we might get
    ///   problems with siturations where we have `new Anti part0 -> unload -> load -> anti part 2`
    ///   as in this case the task calling load would not poll the future be we might have to
    ///   but cant ... so for now we don't add this feature until we absolutely need it
    unload_prevention: AtomicUsize,
}

/// The internal state of a Resource
enum ResourceState {
    /// The resource is not loaded
    ///
    /// # Constraint
    /// This state should only appear in `Resources` which have a `Source`
    NotLoaded,

    /// In the process of loading a resource
    LoadingBuffer(sync_helper::MutOnly<SendBoxFuture<FileBuffer, Error>>),

    /// The resource is "loaded" but not encoded, i.e. wrt. the outer API
    /// loading is not yet complete
    Loaded(FileBuffer),

    /// In the process of transfer encoding which is part of loading a resource
    EncodingBuffer(sync_helper::MutOnly<SendBoxFuture<TransferEncodedFileBuffer, Error>>),

    /// The resource is complete loaded (including transfer encoding)
    TransferEncoded(TransferEncodedFileBuffer),

    /// Loading the resource failed
    Failed
}

/// A lock guard for the `TransferEncodedFileBuffer` contained in a Resource
///
/// This a basically a workaround for not having a `RwLockReadGuard::map` method.
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

/// Future driving the (internal) loading of a Resource resolving to a `ResourceAccessGuard`
#[derive(Debug)]
pub struct ResourceLoadingFuture<C: BuilderContext> {
    /// makes sure the Resource is keept alive and allows us to access/poll it
    inner: Arc<ResourceInner>,
    /// the context we use to 1. "load" the resource data,  2. offload the encoding
    ctx: C,
    /// which state the future is in, neede to synchronize the polling so that only
    /// one future at a time actually polls the same resource
    poll_state: PollState,
    /// the `ResourceAccessGuard` we return iff the loading succeed and discard elsewise
    /// (it's creation lets us determine if we have to poll, or if someone else does
    /// the actual polling)
    anti_unload: Option<ResourceAccessGuard>
}

/// State of the `ResourceLoadingFuture`
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
enum PollState {
    /// the `ResourceLoadingFuture` was created but poll was not yet called
    NotPolled,

    /// we can, and need to, poll the state machine in the resource driving it to completation
    CanPoll,

    /// we are not allowed to poll the state machine in the resource as some one else does so.
    ///
    /// As the inner state machine of a resource could be polled by multiple futures from multiple
    /// tasks/threads at the same time, we need to synchronize it. The `RwLock` gives us already
    /// some synchronization but there is one problem, if task T1 polls it and gets a `NotReady` it
    /// will be parked until notified and the future polled in the state machine will remember to
    /// notify T1, but if then T2 polls (and e.g. also gets `NotReady`) the inner future in the
    /// state machine will forget that it has to notify T1 instead it will now remember to notify
    /// T2, which means T1 stays parked for ever.
    ///
    /// So we make sure that for any Resource there is only one Future which has the poll state
    /// `CanPoll` and all others have the state `SomeOneElsePolls`.
    SomeOneElsePolls,

    /// the future was resolved and is done
    Done
}

#[derive(Debug)]
pub struct ResourceAccessGuard {
    handle: Arc<ResourceInner>,
}



impl Resource {

    /// create a new resource from a `ResourceState` and optionally a `Source`
    ///
    /// This is meant to be used by other constructors and is not public, also
    /// not all combinations of state and source are valide, i.e. a `NotLoaded`
    /// state requires a resource to have a `Source`.
    ///
    /// # Panics
    ///
    /// In debug mode this panics if called with a state which  is `NotLoaded` and
    /// no source. It's ok to only panic in debug mode as this is an private constructor,
    /// the constructors using it have to make sure this constraint is uphold.
    fn _new(state: ResourceState, source: Option<Source>) -> Self {
        let state_info = state.derive_state_info();
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

    /// Create a new `Resource` from a `Source`
    ///
    /// Use `create_loading_future` to drive the internal loading of the resource and
    /// so that it's data can then be accessed
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
    pub fn sourceless_from_future(fut: SendBoxFuture<FileBuffer, Error>) -> Self {
        Self::_new( ResourceState::LoadingBuffer(sync_helper::MutOnly::new(fut)), None)
    }


    /// returns the state info of this Resource
    ///
    /// The state info represents the inner state of the Resource, e.g.
    /// if it is loading, loaded, loading failed, etc.
    ///
    /// # Blocking?
    ///
    /// No
    pub fn state_info(&self) -> ResourceStateInfo {
        self.inner.state_info()
    }

    /// returns true if the resource is loaded
    ///
    /// Note that loaded means it is completly loaded including transfer encoding.
    ///
    /// # Blocking?
    ///
    /// No
    pub fn is_loaded(&self) -> bool {
        self.inner.is_loaded()
    }

    /// Returns `Some` Guard to a `TransferEncodedFileBuffer` if the resource is loaded, `None` else wise
    ///
    /// # Blocking?
    ///
    /// Yes, it will block to get a read lock on the inner resource, but before it it will use
    /// non-blocking methods to make sure the resource is loaded. So blocking only appear if
    /// in between the usage of the non-blocking methods and aquiring the read lock the resource
    /// was started to beeing unloaded (which is basically just droping stuf) and only would be
    /// blocked for this short time frame. (Well and theoretically it could be both unloaded
    /// and started to be loaded in between the atomic check and the lock aqusation in which
    /// case it could block for the time of a poll call, but thats kind of unlikely and still
    /// should not take to long)
    pub fn get_if_encoded( &self ) -> Option<Guard> {
        self.inner.get_if_encoded()
    }

    /// creates a `ResourceLoadingFuture` which can drive the internal loading of the `Resource`
    ///
    /// It will resolve to a `ResourceAccessGuard` which prevents the `Resource` from beeing unloaded
    /// while alive and which allows easy access to the underling `TransferEncodedFileBuffer`
    ///
    /// # Example
    /// ```
    /// # extern crate futures;
    /// # extern crate mail_type;
    /// # use mail_type::{Resource, ResourceAccessGuard};
    /// # use mail_type::context::BuilderContext;
    /// # use futures::Future;
    /// fn load_resource_blocking<C>(resource: &Resource, ctx: C) -> ResourceAccessGuard
    ///     where C: BuilderContext
    /// {
    ///     resource.create_loading_future(ctx).wait().expect("loading failed")
    /// }
    /// # fn main() {}
    /// ```
    pub fn create_loading_future<C>(&self, ctx: C) -> ResourceLoadingFuture<C>
        where C: BuilderContext
    {
        ResourceLoadingFuture::new(self.clone(), ctx)
    }


    /// get the `Source` of a `Resource` (if any)
    ///
    /// # Blocking?
    ///
    /// No.
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
    /// between a call to `try_unload` and a immediately following call to `state_info` the
    /// resource was already loaded again (or is in the process of being loaded).
    ///
    /// # Error
    ///
    /// Unloading a resource can fail in follwing cases:
    /// 1. it does not have a source, as we can not reload it
    ///    only droping all `Resource`'s sharing the same inner resource
    ///    will free the memory
    /// 2. The resource is prevented from beeing unloaded by having one or more
    ///    `ResourceAccessGuards` alive, which normally also means it is intended to
    ///    be used any moment
    /// 3. It can't get the write lock, which can happen if someone is currently
    ///    having a `guard` on it, i.e. somone uses it
    ///
    /// # Blocking
    ///
    /// This method does not block, through it can block other
    /// mothods as it does _try_ to aquire the write lock to
    /// the inner resources state
    pub fn try_unload(&self) -> Result<(), ResourceError> {
        self.inner.try_unload()
    }

}

/// returns the write lock of a RwLock ignoring any poisoning if possible
///
/// (semantics like `RwLock::try_read ` but without poison)
fn try_read_lock_poisonless<T>(lock: &RwLock<T>) -> Option<RwLockReadGuard<T>> {
    use std::sync::TryLockError::*;
    match lock.try_read() {
        Ok(lock) => Some(lock),
        Err(Poisoned(plock)) => Some(plock.into_inner()),
        Err(WouldBlock) => None
    }
}

fn read_lock_poisonless<T>(lock: &RwLock<T>) -> RwLockReadGuard<T> {
    match lock.read() {
        Ok(lock) => lock,
        Err(plock) => plock.into_inner(),
    }
}

/// returns the read lock of a RwLock ignoring any poisoning if possible
///
/// (semantics like `RwLock::try_write ` but without poison)
fn try_write_lock_poisonless<T>(lock: &RwLock<T>) -> Option<RwLockWriteGuard<T>> {
    use std::sync::TryLockError::*;
    match lock.try_write() {
        Ok(lock) => Some(lock),
        Err(Poisoned(plock)) => Some(plock.into_inner()),
        Err(WouldBlock) => None
    }
}


impl ResourceInner {

    fn is_loaded(&self) -> bool {
        self.state_info() == ResourceStateInfo::Loaded
    }


    fn state_info(&self) -> ResourceStateInfo {
        self.state_info.get()
    }


    /// use this to set the state including a state infor derived from the state
    fn try_modify_state_if<P, F, R, E>(&self, predicate: P, modif: F) -> Option<StdResult<R, E>>
        where F: FnOnce(ResourceState) -> StdResult<(ResourceState, R), E>,
              P: FnOnce(&ResourceState) -> bool
    {
        return try_write_lock_poisonless(&self.state)
            .and_then(|mut guard| {
                if predicate(&*guard) {
                    let _unwind_safety = FailInfoOnePanic(&self.state_info);
                    let state = mem::replace(&mut *guard, ResourceState::Failed);
                    match modif(state) {
                        Ok((new_state, paiload)) => {
                            let state_info = new_state.derive_state_info();
                            *guard = new_state;
                            self.state_info.set(state_info);
                            Some(Ok(paiload))
                        },
                        Err(e) => {
                            self.state_info.set(ResourceStateInfo::Failed);
                            Some(Err(e))
                        }
                    }
                } else {
                    None
                }
            });

        // we only need this for one edge case in which a call to `ResourceLoadingFuture::poll`
        // did panic but the future was _not_ dropped _and_ there is another `ResourceLoadingFuture`
        // polling the same resource (or more correctly waiting for it to be done)
        struct FailInfoOnePanic<'a>(&'a AtomicStateInfo);
        impl<'a> Drop for FailInfoOnePanic<'a> { fn drop(&mut self) { if ::std::thread::panicking() {
            self.0.set(ResourceStateInfo::Canceled)
        }}}
    }

    fn set_state_info(&self, info: ResourceStateInfo) {
        self.state_info.set(info)
    }

    /// Tries to be the one to continue polling from a canceled state.
    ///
    /// If the state (state info state!) is not canceled this will return the
    /// current `ResourceStateInfo` state.
    ///
    /// If the state was `ResourceStateInfo::Canceled` then it now is
    /// `ResourceStateInfo::Loading` and `ResourceStateInfo::Canceled` is returned.
    ///
    /// **If `Canceled` is returned (i.e. it _was_ canceled)  the caller of this
    /// function is now responsible for driveing the resource inner state to completion /
    /// to beeing loaded**
    fn try_continue_from_cancel(&self) -> ResourceStateInfo {
        self.state_info.try_continue_from_cancel()
    }


    /// access the source of the resource (if any)
    fn source(&self) -> Option<&Source> {
        self.source.as_ref()
    }

    fn try_unload(&self) -> Result<(), ResourceError> {
        use self::ResourceStateInfo::*;
        match self.state_info() {
            NotLoaded => Ok(()),
            //TODO typed error
            Loading => Err("resource is in use, can't unload it".into()),
            Loaded | Canceled | Failed => self._try_unload()
        }
    }

    fn _try_unload(&self) -> Result<(), ResourceError> {
        if self.source().is_some() {
            // NOTE: we relay on 3 thinks here:
            //  1. loading/polling uses the lock to change the state
            //  2. we only create inital AccessGuards while holding the guard (ro is ok)
            //  3. we use a guard and check in the guard if there are AccessGuards
            //  If this would be lock free thinks would become much more more complex
            //  (somethink on the line of first publish that it's not loaded, then
            //   see if someone might have potentially gotten a AccessGuard by super
            //   fast polling, if so do set the info back to Not Loaded, only if not
            //   change the actual state, and more)
            if 0 == self.unload_prevention.load(Ordering::Acquire) {
                let res = self.try_modify_state_if(
                    // there might have been a load/unload prevention before we got the lock
                    |_| 0 == self.unload_prevention.load(Ordering::Acquire) ,
                    // we do not care which state it was, now it's not loaded
                    |_current| Ok((ResourceState::NotLoaded, ()))
                );
                if let Some(res) = res {
                    return res;
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

        // we do only try to get the lock if state_info is Loaded,
        // it it is there should be no write access to it and as such
        // this should not fail, except if we currently are unloading it,
        // in which case failing is what we want
        try_read_lock_poisonless(&self.state)
            .and_then(|state_guard| {
                let ptr = match *state_guard {
                    TransferEncoded( ref encoded )  =>
                        Some( encoded as *const TransferEncodedFileBuffer ),
                    _ => None
                };
                ptr.map(|ptr| {
                    Guard {
                        guard: state_guard,
                        state_ref: ptr,
                        _marker: PhantomData
                    }
                })
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

    /// generate a state info from the current state
    ///
    /// This can not return `ResourceStateInfo::Canceled` as cancelation is
    /// not expressed in the `ResourceState` but only in the `ResourceStateInfo`
    fn derive_state_info(&self) -> ResourceStateInfo {
        use self::ResourceState::*;
        match *self {
            NotLoaded => ResourceStateInfo::NotLoaded,
            TransferEncoded(..) => ResourceStateInfo::Loaded,
            Failed => ResourceStateInfo::Failed,
            _ => ResourceStateInfo::Loading
        }
    }

    /// drive the "loading" of the resource by consuming the state and generating a new one
    ///
    /// This will drive the state machine until it calls poll on a future contained in the
    /// state maching and the call to poll returns `NotRead` or an error (or if the inner
    /// state was `Failed` or reached completation)
    ///
    /// It requires a `ctx` as it will load a resources data using `ctx.load_resource`
    /// and offloads the transfer encoding of the data with `ctx.offload_fn`/`ctx.offload`
    fn poll_encoding_completion<C>(self, source: &Option<Source>, ctx: &C)
                                   -> Result<(ResourceState, Async<()>), ResourceError>
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
                        .expect("[BUG] illegal state no source and not loaded");

                    LoadingBuffer(sync_helper::MutOnly::new(ctx.load_resource(source)))
                },

                LoadingBuffer(mut fut) => {
                    let async = fut
                        .get_mut().poll()
                        .map_err(|err| err
                            .with_source_iri_or_else(|| {
                                source.as_ref().map(|source| source.iri.clone())
                            })
                        )?;

                    match async {
                        Async::Ready(buf)=> Loaded(buf),
                        Async::NotReady => {
                            return Ok((
                                LoadingBuffer(fut),
                                Async::NotReady
                            ))
                        }
                    }
                },

                Loaded(buf) => {
                    EncodingBuffer(sync_helper::MutOnly::new(ctx.offload_fn(move || {
                        TransferEncodedFileBuffer::encode_buffer(buf, None)
                    })))
                },

                EncodingBuffer(mut fut) => {
                    match fut.get_mut().poll()? {
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
                    let iri = source.as_ref()
                        .map(|source| source.iri.clone());

                    return Err(ResourceLoadingError::from((
                        iri,
                        ResourceLoadingErrorKind::SharedResourcePoison
                    )).into());
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

/// We need to implement `BodyBuffer` to use `Resource` as a body buffer for encoding
/// a `Mail`
impl BodyBuffer for Resource {
    fn with_slice<FN, R>(&self, func: FN) -> Result<R, EncodingError>
        where FN: FnOnce(&[u8]) -> Result<R, EncodingError>
    {
        if let Some( guard ) = self.get_if_encoded() {
            func(&*guard)
        } else {
            Err(EncodingErrorKind::AccessingMailBodyFailed.into())
        }

    }
}

// we can't have one make_done function due to borrow checker conflicts on
// borrowing all of self mut
macro_rules! make_done {
    ($self:ident) => (
        make_done!(&mut $self.poll_state, &mut $self.anti_unload)
    );
    ($poll_state:expr, $anti_unload:expr) => ({
        *$poll_state = PollState::Done;
        let anti = $anti_unload.take()
            .expect("[BUG] anti is always set when polling started, only removed once it ends");
        Async::Ready(anti)
    });
}

impl<C> ResourceLoadingFuture<C>
    where C: BuilderContext
{
    /// creates a new future from a resource and a context
    fn new(resource: Resource, ctx: C) -> Self {
        ResourceLoadingFuture {
            inner: resource.inner, ctx,
            poll_state: PollState::NotPolled,
            anti_unload: None
        }
    }

    /// helper function called by `<Self as Future>::poll`
    fn _poll_inner(&mut self)
        -> Poll<ResourceAccessGuard, Error>
    {
        if self.inner.is_loaded() {
            // can we rely on it actually beeing loaded?
            // 1. in which case might it not be the case
            // => if we unloaded it
            // => unloading it sets the state using a Release barier,
            // => we have a Aquire bariere here
            // => so can our Aquire barier move _above_ the release barier?
            //  => if so than wo could still see it loaded
            //   => are you sure there is still the anti_unload which we did create,
            //      which uses AcqRel in fetch_add and prevents unloading i.e.
            //      if we do have a anti_unload then we can not see at Loaded which is
            //      no longer up to date
            //  => if not so then this can not happen
            // ===> it's ok
            // NOTE: that we can only do this because we hold an AntiUnloadGuard while
            //       loading else this would be bad
            Ok(make_done!(self))
        } else {
            // as we can not partially borrow self mut we have to pass the references
            // to the fields instead of passing self
            let &mut ResourceLoadingFuture {
                ref inner, ref ctx,
                ref mut poll_state, ref mut anti_unload
            } = self;

            // try to get a lock, if this fails we either:
            // 1. have a loaded resource, in which case we do not need the lock at all [mainly]
            // 2. some one did not use the state_info to query some info [ok, np]
            // 3. there is a bug and two features poll the resource [well, it is at last still safe]
            let res = inner.try_modify_state_if(
                // we are polling, so not if here (it would be relevant if we tranform Resource
                // to a lock less algorithm, which is quite doable, but as long as we do not
                // need this we don't do so as it's easy to mess up custom synchronizations using
                // Atomics
                |_| true,
                |state| {
                    ResourceLoadingFuture::poll_inner_with_state(
                        state,
                        &inner.source, ctx, poll_state, anti_unload
                    )
                }
            );

            // we did got the guard
            if let Some(res) = res {
                res
            } else {
                // Should not happen. All info methods use the state_info atomic.
                // But if it does, we know it's not long term (as it is not loaded)
                // and just try again next tick
                task::current().notify();
                Ok(Async::NotReady)
            }
        }
    }

    /// drive the inner state machine
   ///
   /// This replaces the state with `Failed` then
   /// uses `state.poll_encoding_completion` to get the new state
   /// and sets it (and the state info generate from it) due to this
   /// Even if there is a panic, it will not cause any bad state, the
   /// state (and state info) will be failed like expected. Because of
   /// this we do not need to bother about lock poisoning at all.
   ///
    fn poll_inner_with_state(
        state: ResourceState,
        source: &Option<Source>,
        ctx: &C,
        poll_state: &mut PollState,
        anti_unload: &mut Option<ResourceAccessGuard>
    ) -> StdResult<(ResourceState, Async<ResourceAccessGuard>), ResourceError>
    {
        let (new_state, async_state) =
            state.poll_encoding_completion(source, ctx)?;

        Ok(match async_state {
            Async::NotReady => (new_state, Async::NotReady),
            Async::Ready(()) => (new_state, make_done!(poll_state, anti_unload))
        })
    }


}


impl<C> Drop for ResourceLoadingFuture<C>
    where C: BuilderContext
{
    /// cancels the future if it is dropped before cancelation
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
    type Item = ResourceAccessGuard;
    type Error = ResourceError;


    fn poll( &mut self ) -> Poll<Self::Item, Self::Error> {

        use self::PollState::*;
        match self.poll_state {
            NotPolled => {
                let (anti, is_initial) = {
                    // we use the lock to sync this with the try_unlod
                    // we don't really need to do this as the only way
                    // to create new AccessGuards once there are none through a future,
                    // but if not we have to get even more atomic interactions right, for
                    // new this is a usable and good enough solution
                    let _guard = read_lock_poisonless(&self.inner.state);
                    let res = ResourceAccessGuard::new(&self.inner, &_guard);
                    mem::drop(_guard);
                    res
                };
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
                    ResourceStateInfo::Loaded => Ok(make_done!(self)),
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
            //Note: panicing if a poll is called after the future is resolved is
            // normal behaviour, actually some future behave much worse then just
            // panicing in that case, through we _could_ rewind the future if we
            // want to make it fused by default
            //TODO typed error
            Done => panic!("[BUG] called poll after future was resolved, use fuse if you have to")
        }
    }
}



/// Keep alive to prevent a resource from being unloaded.
///
/// This also provides easy access to the `TransferEncodedFileBuffer` contained
/// in the `Resource` once it is loaded
impl ResourceAccessGuard {

    /// Creates a new `ResourceAccessGuard` returning it and a bool indicating if it's the first guard.
    ///
    /// who ever creates a ResourceAccessGuard which is the first is responsible for loading the
    /// resource it's belongs to (no one else will do so, at last not until the given guard
    /// and all others created after it are dropped).
    ///
    /// # Context
    ///
    /// This needs to be called why the inner RwLock is hold, or it can conflict with
    /// `try_unload`
    fn new(resource: &Arc<ResourceInner>, _guard: &RwLockReadGuard<ResourceState>)
        -> (Self, bool)
    {
        let handle = resource.clone();
        let prev = handle.unload_prevention.fetch_add(1, Ordering::AcqRel);
        (ResourceAccessGuard { handle }, prev == 0)
    }

    /// return a Guard to the underlying `TransferEncodedFileBuffer`
    //NOTE: this would panic if called before the resource was loaded, but any
    // `ResourceAccessGuard` will only exposed to pub once a resource was loaded succsefully.
    //(before it will lay dormant in the `ResourceLoadFuture` where we need it to prevent
    // interchanging double loads)
    pub fn access(&self) -> Guard {
        self.handle.get_if_encoded()
            .expect("[BUG] should only be accessible when loaded succesfull")
    }

}

impl Clone for ResourceAccessGuard {

    fn clone(&self) -> Self {
        let handle = self.handle.clone();
        //NOTE: it might be possible to make this Release but
        //      this could play bad wrt. try_unload if another
        //      fetch_sub(1, Release) (mainly of this thread, wiht
        //      synced AccessGuards not only) could be ordered before this
        //      fetch, braking the guarntee that the result of this
        //      fetch_add can not be 1 as it need another AccessGuard
        //      to exist. I'm pretty sure it would be ok to use Relese
        //      but I would have to take a deeper and time consuming look
        //      at the standard to make sure this really is the case.
        handle.unload_prevention.fetch_add(1, Ordering::AcqRel);
        ResourceAccessGuard { handle }
    }
}

impl Drop for ResourceAccessGuard {

    fn drop(&mut self) {
        // decrease the unload_prevention count, as we added one when construction and no one else
        // does access the variable this won't underflow
        self.handle.unload_prevention.fetch_sub(1, Ordering::AcqRel);
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


    trait AssertSend: Send {}
    impl AssertSend for Resource {}

    trait AssertSync: Sync {}
    impl AssertSync for Resource {}
}


mod sync_helper {
    pub(crate) struct MutOnly<T> {
        //CONSTRAINT: this can only be accessed through a &mut borrow
        inner: T
    }

    impl<T> MutOnly<T> {
        pub(crate) fn new(inner: T) -> Self {
            MutOnly { inner }
        }

        pub(crate) fn get_mut(&mut self) -> &mut T {
            &mut self.inner
        }
    }

    //SAFE: this is safe as the data can only be accessed &mut, i.e. from one place at a time
    // which means there can't be multiple references to it between thread and as such it's
    // Sync even if the inner data is "just" Send (it can be seen that bettween each mut access,
    // wich btw. would need some form of synchronization, the data is Send to the thread where it's
    // accessed from). It's the same reason as why Mutex<T> is `Sync` even if T is not `Sync` but
    // just `Send`
    unsafe impl<T: Send> Sync for MutOnly<T> {}
}
