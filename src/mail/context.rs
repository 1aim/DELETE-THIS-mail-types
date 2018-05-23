use std::sync::Arc;
use std::fmt::Debug;

use futures::{ future, Future, IntoFuture };
use utils::SendBoxFuture;

use ::error::ResourceLoadingError;
use ::headers::components::{MediaType, MessageId, ContentId};
use ::file_buffer::FileBuffer;
use ::iri::IRI;

/// POD containing the path from which a resource should be loaded as well as and
/// optional media_type and name
#[derive( Debug, Clone )]
pub struct Source {

    /// A International Resource Identifier pointing to a source
    /// from which the Resource can be loaded. Note that the interpretation
    /// of the IRI is left to the `ResourceLoader` implementation of the
    /// context. The `ResourceLoader` can decide to reject valid IRI's e.g.
    /// a (non local) http url is likely to be rejected by any implementation.
    pub iri: IRI,

    /// allows providing a explicit media type, if `None` the
    /// media type could be sniffed or retrived from meta data
    /// associated with the `source` IRI. It also possible that
    /// Not providing a Media Type can lead to an error if it
    /// can not be retrieved through another mean.
    pub use_media_type: Option<MediaType>,

    /// allows providing a explicit name, if not provided the
    /// name is normally derived from the `source` IRI
    pub use_name: Option<String>
}

// Future versions could consider allowing non static non clone context
// making Resource::create_load_future keeping a reference to the resource
// etc. BUT this is much more of a hassel to work with and to integrate with
// e.g. tokio/futures
/// # Clone / Send / Sync
///
/// `Context` are meant to be easily shareable, cloning them should be
/// cheap, as such if a implementor contains state it might make sense for an
/// implementor to have a outer+inner type where the inner type is wrapped
/// into a `Arc` e.g. `struct SomeCtx { inner: Arc<InnerSomeCtx> }`.
pub trait Context: Clone + Send + Sync + 'static {

    /// returns a Future resolving to a FileBuffer.
    ///
    /// If a name is provided the given name should be used in the `FileMeta`,
    /// even if there is another name associated with the IRI for the
    /// `ResourceLoader` implementation.
    ///
    /// If a media type is provided it should be used as the media type
    /// for the result, if non media type is provided the implementor can
    /// decide to trigger an error or, if possible, find the media type by
    /// it self.
    ///
    /// # Media Type usage considerations
    ///
    /// If the implementation load resources from a source with contains media types and
    /// a media type is provided it should check the provided one and the predefined one
    /// for compatibility. The proved media type should be preferred over the predefined one.
    ///
    /// If the implementation decides to sniff the media type it
    /// should do so in a cautious way, especially for text media types sniffing
    /// can be quite unreliable (e.g. any JSON is also a Toml file, any text
    /// could happen to be valid Toml a Toml file could happen to be valid
    /// php/python config file, too etc.)
    ///
    /// # Async considerations
    ///
    /// The future is directly polled hen resolving a Resource,
    /// which means it's directly polled when turning a Mail into an Encodeable mail.
    /// As such if the loading process blocks or consumes to many cpu resources it's
    /// recommended to offload the work to e.g. a CpuPool, if the implementor of this
    /// trait also implements RunElsewhere it simple doable by using `RunElsewhere::execute`.
    fn load_resource(&self, &Source) -> LoadResourceFuture;

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

pub type LoadResourceFuture = SendBoxFuture<FileBuffer, ResourceLoadingError>;


pub trait ResourceLoaderComponent: Debug + Send + Sync + 'static {

    fn load_resource<O>( &self, source: &Source, offload: &O) -> LoadResourceFuture
        where O: OffloaderComponent;
}

pub trait OffloaderComponent: Debug + Send + Sync + 'static {
    fn offload<F>(&self, fut: F) -> SendBoxFuture<F::Item, F::Error>
        where F: Future + Send + 'static,
              F::Item: Send+'static,
              F::Error: Send+'static;
}

pub trait MailIdGenComponent: Debug + Send + Sync + 'static {

    //TODO doc: link method
    /// returns the same unique message id
    ///
    /// see `Context::generate_message_id` for more details
    fn generate_message_id(&self) -> MessageId;

    //TODO doc: link method
    /// generates a new content id
    ///
    /// see `Context::generate_content_id` for more details
    fn generate_content_id(&self) -> ContentId;
}


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
    pub fn new(resource_loader: R, offloader: O, message_id_gen: M) -> Self {
        CompositeContext {
            inner: Arc::new((resource_loader, offloader, message_id_gen)),
        }
    }

    pub fn resource_loader(&self) -> &R {
        &self.inner.0
    }

    pub fn offloader(&self) -> &O {
        &self.inner.1
    }

    pub fn id_gen(&self) -> &M {
        &self.inner.2
    }
}

impl<R, O, M> Context for CompositeContext<R, O, M>
    where R: ResourceLoaderComponent,
          O: OffloaderComponent,
          M: MailIdGenComponent
{

    fn load_resource(&self, source: &Source) -> LoadResourceFuture {
        self.resource_loader().load_resource(source, self.offloader())
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