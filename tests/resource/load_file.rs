use std::path::Path;

use futures::{Future, future};

use mail_codec::file_buffer::FileBuffer;
use mail_codec::{
    MediaType,
    Resource, ResourceSpec, ResourceState,
};
use mail_codec::context::CompositeBuilderContext;
use mail_codec::default_impl::{ FSFileLoader, simple_cpu_pool };

macro_rules! context {
    () => ({
        use std::env;
        CompositeBuilderContext::new(
            FSFileLoader::new(
                env::current_dir().unwrap()
                    .join(Path::new("./tests/test-resources/"))
            ),
            simple_cpu_pool()
        )
    });
}

fn loaded_resource(path: &str, media_type: &str, name: Option<&str>) -> Resource {
    let spec = ResourceSpec {
        path: Path::new(path).to_owned(),
        media_type: MediaType::parse(media_type).unwrap(),
        name: name.map(|s|s.to_owned()),
    };
    let mut resource = Resource::from_spec(spec);
    let ctx = context!();

    future::poll_fn(|| {
        resource.poll_encoding_completion(&ctx)
    }).wait().unwrap();

    assert_eq!(resource.state(), ResourceState::EncodedFileBuffer);
    resource
}


#[test]
fn get_name_from_path() {
    let resource =
        loaded_resource("img.png", "image/png", None);

    let tenc_buffer = resource.get_if_encoded()
        .expect("no problems witht the lock")
        .expect("it to be encoded");

    let fbuf: &FileBuffer  = &**tenc_buffer;

    assert_eq!(fbuf.file_meta().file_name, Some("img.png".to_owned()));
}

#[test]
fn use_name_is_used() {
    let resource =
        loaded_resource("img.png", "image/png", Some("That Image"));

    let tenc_buffer = resource.get_if_encoded()
        .expect("no problems witht the lock")
        .expect("it to be encoded");

    let fbuf: &FileBuffer  = &**tenc_buffer;

    assert_eq!(fbuf.file_meta().file_name, Some("That Image".to_owned()));
}

