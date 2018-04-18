use std::sync::Arc;
use std::fmt::Debug;

use futures::{ future, Future, IntoFuture };
use utils::SendBoxFuture;

use ::error::ResourceLoadingError;
use ::MediaType;
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
/// `BuilderContext` are meant to be easily shareable, cloning them should be
/// sheap, as such if a implementor contains state it might make sense for an
/// implementor to have a outer+inner type where the inner type is wrapped
/// into a `Arc` e.g. `struct SomeCtx { inner: Arc<InnerSomeCtx> }`.
pub trait BuilderContext: Clone + Send + Sync + 'static {

    /// returns a Future resolving to a FileBuffer.
    ///
    /// If a name is provided the given name should be used in the `FileMeta`,
    /// even if there is another name associated with the IRI for the
    /// `ResourceLoader` implementation.
    ///
    /// If a media type is provided it should be used as the media type
    /// for the result, if non media type is provided the implementor can
    /// decide to trigger an error or, if possible, find the media type by
    /// it'self.
    ///
    /// # Media Type usage consiferations
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
    fn load_resource( &self, &Source) -> LoadResourceFuture;

    /// offloads the execution of the future `fut` to somewhere else e.g. a cpu pool
    fn offload<F>(&self, fut: F) -> SendBoxFuture<F::Item, F::Error>
        where F: Future + Send + 'static,
              F::Item: Send+'static,
              F::Error: Send+'static;

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


#[derive(Debug)]
pub struct CompositeBuilderContext<
    R: ResourceLoaderComponent,
    O: OffloaderComponent
>(Arc<InnerContext<R,O>>);

impl<R, O> Clone for CompositeBuilderContext<R, O>
    where R: ResourceLoaderComponent, O: OffloaderComponent
{
    fn clone(&self) -> Self {
        CompositeBuilderContext(self.0.clone())
    }
}

#[derive(Debug)]
struct InnerContext<R: ResourceLoaderComponent, O: OffloaderComponent> {
    resource_loader: R,
    offloader: O
}

impl<R, O> CompositeBuilderContext<R, O>
    where R: ResourceLoaderComponent,
          O: OffloaderComponent
{
    pub fn new(resource_loader: R, offloader: O) -> Self {
        CompositeBuilderContext(Arc::new(InnerContext {
            resource_loader, offloader
        }))
    }

    pub fn resource_loader(&self) -> &R {
        &self.0.resource_loader
    }

    pub fn offloader(&self) -> &O {
        &self.0.offloader
    }
}

impl<R, O> BuilderContext for CompositeBuilderContext<R, O>
    where R: ResourceLoaderComponent,
          O: OffloaderComponent
{

    fn load_resource( &self, source: &Source) -> LoadResourceFuture {
        self.0.resource_loader.load_resource(source, &self.0.offloader)
    }

    fn offload<F>(&self, fut: F) -> SendBoxFuture<F::Item, F::Error>
        where F: Future + Send + 'static,
              F::Item: Send+'static,
              F::Error: Send+'static
    {
        self.0.offloader.offload(fut)
    }
}