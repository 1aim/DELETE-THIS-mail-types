//! Provides the context needed for building/encoding mails.
use std::sync::Arc;
use std::fmt::Debug;

use futures::{ future, Future, IntoFuture };
use utils::SendBoxFuture;

use headers::header_components::{
    MessageId, ContentId
};

use ::error::ResourceLoadingError;
use ::resource::{Source, Data, EncData};

/// This library needs a context for creating/encoding mails.
///
/// The context is _not_ meant to be a think you create once
/// per mail but something you create once one startup and then
/// re-user wherever it is needed in your application.
///
/// A context impl. provides following functionality to this library:
///
/// 1. Load a `Resource` based on a `Source` instance.
///    I.e. an IRI plus some optional meta information.
/// 2. Generate an unique message id. This should be
///    an world unique id to comply with the standard(s).
/// 3. Generate an unique content id. This should be
///    an world unique id to comply with the standard(s).
/// 4. A way to "offload" work to some other "place" e.g.
///    by scheduling it in an thread pool.
///
/// The `CompositeContext` provides a impl. for this trait
/// which delegates the different tasks to the components
/// it's composed of. This allow to reuse e.g. the message
/// if generation and offloading but use a custom resource
/// loading. Generally it's recommended to use the `CompositeContext`.
///
/// There is also an default implementation (using the
/// `CompositeContext` in the `default_impl` module).
///
/// # Clone / Send / Sync / 'static ?
///
/// `Context` are meant to be easily shareable, cloning them should be
/// cheap, as such if a implementor contains state it might make sense for an
/// implementor to have a outer+inner type where the inner type is wrapped
/// into a `Arc` e.g. `struct SomeCtx { inner: Arc<InnerSomeCtx> }`.
pub trait Context: Debug + Clone + Send + Sync + 'static {


    /// Loads and transfer encodes a `Data` instance.
    ///
    /// This is called when a `Mail` instance is converted into
    /// a encodable mail an a `Resource::Data` instance is found.
    ///
    /// This can potentially use the IRI to implement a simple
    /// caching scheme potentially directly caching the transfer
    /// encoded instance skipping the loading + encoding step for
    /// common resources like e.g. a logo.
    ///
    /// # Async Considerations
    ///
    /// This function should not block and schedule the encoding
    /// in some other place e.g. by using the contexts offload
    /// functionality.
    fn load_resource(&self, source: &Source)
        -> SendBoxFuture<EncData, ResourceLoadingError>;

    /// Transfer encodes a `Data` instance.
    ///
    /// This is called when a `Mail` instance is converted into
    /// a encodable mail an a `Resource::Data` instance is found.
    ///
    /// The default impl. of this function just calls
    /// `data.transfer_encode(data, Default::default())` but a more
    /// sophisticated implementation could use the `Data`s content id
    /// for some caching scheme e.g. a LRU cache. Which can safe the
    /// encoding step for commonly used resources like e.g. a logo.
    ///
    /// # Async Considerations
    ///
    /// This function should not block and schedule the encoding
    /// in some other place e.g. by using the contexts offload
    /// functionality.
    fn transfer_encode_resource(&self, data: &Data)
        -> SendBoxFuture<EncData, ResourceLoadingError>
    {
        let data = data.clone();
        self.offload_fn(move || Ok(data.transfer_encode(Default::default())))
    }

    /// generate a unique content id
    ///
    /// As message id's are used to reference messages they should be
    /// world unique this can be guaranteed through two aspects:
    ///
    /// 1. using a domain you own/control on the right hand side
    ///    of the `@` will make sure no id's from other persons/companies/...
    ///    will collide with your ids
    ///
    /// 2. using some internal mechanism for the left hand side, like including
    ///    the time and an internal counter, not that you have to make sure this
    ///    stays unique even if you run multiple instances or restart the current
    ///    running instance.
    ///
    fn generate_message_id(&self) -> MessageId;

    /// generate a unique content id
    ///
    /// Rfc 2045 states that content id's have to be world unique,
    /// while this really should be the case if it's used in combination
    /// with an `multipart/external` or similar body for it's other usage
    /// as reference for embeddings it being mail unique tends to be enough.
    ///
    /// As content id and message id are treated mostly the same wrt. the
    /// constraints applying when generating them this can be implemented
    /// in terms of calling `generate_message_id`.
    fn generate_content_id(&self) -> ContentId;

    //TODO[futures/v>=0.2]: integrate this with Context
    /// offloads the execution of the future `fut` to somewhere else e.g. a cpu pool
    fn offload<F>(&self, fut: F) -> SendBoxFuture<F::Item, F::Error>
        where F: Future + Send + 'static,
              F::Item: Send + 'static,
              F::Error: Send + 'static;

    //TODO[futures/v>=0.2]: integrate this with Context
    /// offloads the execution of the function `func` to somewhere else e.g. a cpu pool
    fn offload_fn<FN, I>(&self, func: FN ) -> SendBoxFuture<I::Item, I::Error>
        where FN: FnOnce() -> I + Send + 'static,
              I: IntoFuture + 'static,
              I::Future: Send + 'static,
              I::Item: Send + 'static,
              I::Error: Send + 'static
    {
        self.offload( future::lazy( func ) )
    }
}


/// Trait needed to be implemented for providing the resource loading parts to a`CompositeContext`.
pub trait ResourceLoaderComponent: Debug + Send + Sync + 'static {

    /// Calls to `Context::load_resource` will be forwarded to this method.
    ///
    /// It is the same as `Context::load_resource` except that a reference
    /// to an `OffloaderComponent` will be passed in as it's likely needed.
    fn load_resource(&self, source: &Source, ctx: &impl Context)
        -> SendBoxFuture<EncData, ResourceLoadingError>;

    /// Calls to `Context::transfer_encode_resource` will be forwarded to this method.
    ///
    /// It is the same as `Context::transfer_encode_resource` except that a reference
    /// to an `OffloaderComponent` will be passed in as it's likely needed.
    fn transfer_encode_resource(&self, data: &Data, ctx: &impl Context)
        -> SendBoxFuture<EncData, ResourceLoadingError>
    {
        let data = data.clone();
        ctx.offload_fn(move || Ok(data.transfer_encode(Default::default())))
    }
}

/// Trait needed to be implemented for providing the offloading parts to a `CompositeContext`.
pub trait OffloaderComponent: Debug + Send + Sync + 'static {

    /// Calls to `Context::offload` and `Context::offload_fn` will be forwarded to this method.
    fn offload<F>(&self, fut: F) -> SendBoxFuture<F::Item, F::Error>
        where F: Future + Send + 'static,
              F::Item: Send+'static,
              F::Error: Send+'static;
}

/// Trait needed to be implemented for providing the id generation parts to a `CompositeContext`.
///
/// It is possible/valid to use the same implementation (internal function etc.) for
/// both message and content ids. While they are used at different places they have
/// mostly the same constraints (their meaning is still different i.e. it's much
/// more important for an message id to be "world unique" then for an content id,
/// expect in some cases where external bodies are used).
pub trait MailIdGenComponent: Debug + Send + Sync + 'static {

    /// Calls to `Context::generate_message_id` will be forwarded to this method.
    fn generate_message_id(&self) -> MessageId;

    /// Calls to `Context::generate_content_id` will be forwarded to this method.
    fn generate_content_id(&self) -> ContentId;
}

/// The `CompositeContext` is the simplest way to get an `Context` implementation.
///
/// Any custom `Context` implementations should be realized through the `CompositeContext`
/// if possible.
///
/// This type consists of 3 components it forward all method calls from `Context` to.
/// This allows the library to have a single `Context` type but match and mix the
/// parts about resource loading, offloading and id generation in whichever way
/// it fits best.
///
/// The composite context will store the components inside of an `Arc` so that
/// it can be easily shared through an application, it also means non of the
/// components have to implement `Clone`.
#[derive(Debug)]
pub struct CompositeContext<
    R: ResourceLoaderComponent,
    O: OffloaderComponent,
    M: MailIdGenComponent
>{
    inner: Arc<(R, O, M)>,
}

impl<R, O, M> Clone for CompositeContext<R, O, M>
    where R: ResourceLoaderComponent,
          O: OffloaderComponent,
          M: MailIdGenComponent
{
    fn clone(&self) -> Self {
        CompositeContext {
            inner: self.inner.clone(),
        }
    }
}

impl<R, O, M> CompositeContext<R, O, M>
    where R: ResourceLoaderComponent,
          O: OffloaderComponent,
          M: MailIdGenComponent
{
    /// Create a new context from the given components.
    pub fn new(resource_loader: R, offloader: O, message_id_gen: M) -> Self {
        CompositeContext {
            inner: Arc::new((resource_loader, offloader, message_id_gen)),
        }
    }

    /// Returns a reference to the resource loader component.
    pub fn resource_loader(&self) -> &R {
        &self.inner.0
    }

    /// Returns a reference to the offloader component.
    pub fn offloader(&self) -> &O {
        &self.inner.1
    }

    /// Returns a reference to the id generation component.
    pub fn id_gen(&self) -> &M {
        &self.inner.2
    }
}

impl<R, O, M> Context for CompositeContext<R, O, M>
    where R: ResourceLoaderComponent,
          O: OffloaderComponent,
          M: MailIdGenComponent
{

    fn load_resource(&self, source: &Source)
        -> SendBoxFuture<EncData, ResourceLoadingError>
    {
        self.resource_loader().load_resource(source, self)
    }

    fn transfer_encode_resource(&self, data: &Data)
        -> SendBoxFuture<EncData, ResourceLoadingError>
    {
        self.resource_loader().transfer_encode_resource(data, self)
    }

    fn offload<F>(&self, fut: F) -> SendBoxFuture<F::Item, F::Error>
        where F: Future + Send + 'static,
              F::Item: Send+'static,
              F::Error: Send+'static
    {
        self.offloader().offload(fut)
    }

    fn generate_content_id(&self) -> ContentId {
        self.id_gen().generate_content_id()
    }

    fn generate_message_id(&self) -> MessageId {
        self.id_gen().generate_message_id()
    }

}

/// Allows using a part of an context as an component.
impl<C> MailIdGenComponent for C
    where C: Context
{
    fn generate_message_id(&self) -> MessageId {
        <Self as Context>::generate_message_id(self)
    }

    fn generate_content_id(&self) -> ContentId {
        <Self as Context>::generate_content_id(self)
    }
}

/// Allows using a part of an context as an component.
impl<C> OffloaderComponent for C
    where C: Context
{
    fn offload<F>(&self, fut: F) -> SendBoxFuture<F::Item, F::Error>
        where F: Future + Send + 'static,
              F::Item: Send+'static,
              F::Error: Send+'static
    {
        <Self as Context>::offload(self, fut)
    }
}

/// Allows using a part of an context as an component.
impl<C> ResourceLoaderComponent for C
    where C: Context
{

    fn load_resource(&self, source: &Source, _: &impl Context)
        -> SendBoxFuture<EncData, ResourceLoadingError>
    {
        <Self as Context>::load_resource(self, source)
    }

    fn transfer_encode_resource(&self, data: &Data, _: &impl Context)
        -> SendBoxFuture<EncData, ResourceLoadingError>
    {
        <Self as Context>::transfer_encode_resource(self, data)
    }
}