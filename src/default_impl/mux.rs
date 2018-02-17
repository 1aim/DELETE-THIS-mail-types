use context::{OffloaderComponent, ResourceLoaderComponent};
use utils::SendBoxFuture;
use error::Error;
use file_buffer::FileBuffer;

type BufferFuture = SendBoxFuture<FileBuffer, Error>;
type DynResourceLoader =
    ResourceLoaderComponent<BufferFuture=BufferFuture>;

pub struct Mux {
    lookup: HashMap<&'static str, Box<DynResourceLoader>>
}

impl Mux {

    pub fn set_handle<R>(&mut self, scheme: &'static str, handle: R)
        where R: ResourceLoaderComponent
    {
        //it would be awesome if we could specialize this for
        // R: ResourceLoaderComponent<BufferFuture=BufferFuture>
        // we actually should be able to using Monomorphization + if with branch elemination etc.
        // let any: Box<Any> = Box::new(Option<R::BufferFutur>::None)
        // any.downcast_ref::<Option<BufferFuture>>()
        //    Ok() => self.lookup.insert(scheme, handle)
        //    Err() => wrap => self.lookup.insert(scheme, wrap)
        let wrapped = Wrapper { inner: handle };
        let boxed: DynResourceLoader = Box::new(wrapped);
        //TODO what happens with previous
        self.lookup.insert(scheme, boxed);
    }
}

impl ResourceLoaderComponent for Mux {
    type BufferFuture = BufferFuture;

    fn load_resource<O>( &self, source: &Source, offload: &O) -> Self::BufferFuture
        where O: OffloaderComponent
    {
        if let Some(handle) = self.lookup.get(source.source.scheme()) {
            handle.load_resource(source, offload)
        } else {
            let err: Error = "unknow iri scheme".into();
            Err(err).into_future().into_boxed()
        }
    }
}


struct Wrapper<I: ResourceLoaderComponent> {
    inner: I
}

impl<I> ResourceLoaderComponent for Wrapper<I>
    where I: ResourceLoaderComponent
{
    type BufferFuture = BufferFuture;

    fn load_resource<O>( &self, source: &Source, offload: &O) -> Self::BufferFuture
        where O: OffloaderComponent
    {
        let original = self.inner.load_resource(source, offload);
        original.into_boxed()
    }
}