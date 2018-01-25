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

fn loaded_resource(path: &str, use_name: Option<&str>, use_mime: Option<&str>) -> Resource {
    let spec = ResourceSpec {
        path: Path::new(path).to_owned(),
        use_name: use_name.map(|s|s.to_owned()),
        use_mime: use_mime.map(|s| MediaType::parse(s).unwrap()),
    };
    let mut resource = Resource::from_spec(spec);
    let ctx = context!();

    future::poll_fn(|| {
        resource.poll_encoding_completion(&ctx)
    }).wait().unwrap();

    assert_eq!(resource.state(), ResourceState::EncodedFileBuffer);
    resource
}

fn _does_sniff(sub_path: &str, content_type: &str) {
    let resource =
        loaded_resource(sub_path, None, None);

    let tenc_buffer = resource.get_if_encoded()
        .expect("no problems witht the lock")
        .expect("it to be encoded");

    let fbuf: &FileBuffer  = &**tenc_buffer;
    assert_eq!(fbuf.content_type().as_str_repr(), content_type);
}

#[test]
fn does_sniff_png_img() {
    _does_sniff("img.png", "image/png")
}

#[test]
fn does_sniff_jpg_img() {
    _does_sniff("img.jpg", "image/jpeg");
}

#[test]
fn does_sniff_pdf_1() {
    _does_sniff("test.pdf", "application/pdf")
}

#[test]
fn does_sniff_pdf_2() {
    _does_sniff("test.pdf", "application/pdf")
}

#[ignore]
#[test]
fn does_sniff_pdf_collision() {
    // I need a pdf with is valid utf8 in a compatible license for this
    // it fails btw. with a pdf I have but can't redistribute...
    // (it detects it as `text/x-tex` as both can start with `%` followed by
    // utf8 (most pdf's are not valid utf8 so this normally does not happen)
    // the problem is that the pdf match still has a (much) higher wight than
    // the other one but still does not get chosen as `tree_magic` does not
    // use wights once it finds a match it's done...
    _does_sniff("/tmp/minimal.pdf", "application/pdf")
}



#[test]
fn get_name_from_path() {
    let resource =
        loaded_resource("img.png", None, None);

    let tenc_buffer = resource.get_if_encoded()
        .expect("no problems witht the lock")
        .expect("it to be encoded");

    let fbuf: &FileBuffer  = &**tenc_buffer;

    assert_eq!(fbuf.file_meta().file_name, Some("img.png".to_owned()));
}

#[test]
fn use_name_is_used() {
    let resource =
        loaded_resource("img.png", Some("That Image"), None);

    let tenc_buffer = resource.get_if_encoded()
        .expect("no problems witht the lock")
        .expect("it to be encoded");

    let fbuf: &FileBuffer  = &**tenc_buffer;

    assert_eq!(fbuf.file_meta().file_name, Some("That Image".to_owned()));
}

#[test]
fn use_mime_is_used() {
    let resource =
        loaded_resource("img.png", None, Some("text/plain; charset=utf8"));

    let tenc_buffer = resource.get_if_encoded()
        .expect("no problems witht the lock")
        .expect("it to be encoded");

    let fbuf: &FileBuffer  = &**tenc_buffer;

    assert_eq!(fbuf.content_type().as_str_repr(), "text/plain; charset=utf8");
}
