use std::path::Path;
use std::env;

use futures::Future;
use soft_ascii_string::SoftAsciiString;
use mail_base::file_buffer::FileBuffer;
use headers::header_components::{MediaType, Domain};
use mail_base::{
    Resource,
    IRI,
    Source,
    ResourceStateInfo
};
use mail_base::context::CompositeContext;
use mail_base::default_impl::{FsResourceLoader, simple_cpu_pool, HashedIdGen, simple_context};

fn dumy_ctx(resource_loader: FsResourceLoader) -> simple_context::Context {
    let domain = Domain::from_unchecked("hy.test".to_owned());
    let unique_part = SoftAsciiString::from_unchecked("w09ad8f");
    let id_gen = HashedIdGen::new(domain, unique_part).unwrap();
    CompositeContext::new(resource_loader, simple_cpu_pool(), id_gen)
}

fn loaded_resource(path: &str, media_type: &str, name: Option<&str>) -> Resource {
    let resource_loader: FsResourceLoader = FsResourceLoader::new(
        env::current_dir().unwrap().join(Path::new("./test_resources/"))
    );

    let ctx = dumy_ctx(resource_loader);


    let source = Source {
        iri: IRI::from_parts("path", path).unwrap(),
        use_media_type: Some(MediaType::parse(media_type).unwrap()),
        use_name: name.map(|s|s.to_owned()),
    };
    let resource = Resource::new(source);

    resource.create_loading_future(ctx).wait().unwrap();

    assert_eq!(resource.state_info(), ResourceStateInfo::Loaded);
    resource
}


#[test]
fn get_name_from_path() {
    let resource =
        loaded_resource("img.png", "image/png", None);

    let tenc_buffer = resource.get_if_encoded()
        .expect("it to be encoded");

    let fbuf: &FileBuffer  = &**tenc_buffer;

    assert_eq!(fbuf.file_meta().file_name, Some("img.png".to_owned()));
}

#[test]
fn use_name_is_used() {
    let resource =
        loaded_resource("img.png", "image/png", Some("That Image"));

    let tenc_buffer = resource.get_if_encoded()
        .expect("it to be encoded");

    let fbuf: &FileBuffer  = &**tenc_buffer;

    assert_eq!(fbuf.file_meta().file_name, Some("That Image".to_owned()));
}

