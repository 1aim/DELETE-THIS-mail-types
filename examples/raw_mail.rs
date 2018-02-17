extern crate mail_codec as mail;
extern crate futures;
extern crate mime;
extern crate futures_cpupool;

use futures::Future;
use futures_cpupool::{Builder as CpuPoolBuilder};

use mail::error::Result;
use mail::prelude::*;
use mail::context::CompositeBuilderContext;
use mail::default_impl::FsResourceLoader;
use mail::file_buffer::FileBuffer;



fn get_some_resource() -> Resource {
    let mt = MediaType::parse("text/plain; charset=utf-8").unwrap();
    let fb = FileBuffer::new(mt, "abcd↓efg".to_owned().into());
    Resource::sourceless_from_buffer(fb)
}

fn main() {
    _main().unwrap();
}

fn _main() -> Result<()> {
    let resource_loader: FsResourceLoader = FsResourceLoader::with_cwd_root().unwrap();
    let builder_ctx = CompositeBuilderContext::new(
        resource_loader,
        CpuPoolBuilder::new().create()
    );

    let mut encoder = Encoder::new( MailType::Ascii );

    let opt_name: Option<&'static str> = None;
    let headers = headers! {
        //FIXME actually use a more realistic header setup
        Subject: "that ↓ will be encoded ",
        MessageId: "ran.a1232.13rwqf23.a@dom",
        From: [
            ("random dude", "this@is.es"),
            ("another person", "abc@def.geh"),
        ],
        Sender: ("random dude", "this@is.es"),
        To: (
            "target@here.it.goes",
            ("some", "thing@nice"),
            ( opt_name, "a@b"),
            ( Some("Uh"), "ee@b"),
            // just writing None wont work due to type inference
            // so either do not use the tuple form or use
            // the NoDisplayName helper
            ( NoDisplayName, "cc@b")
        ),
        ReturnPath: None
    }?;
    let mail = Builder::singlepart( get_some_resource() )
        .headers( headers )?
        .build()?;

    let encodable_mail = mail.into_encodeable_mail( &builder_ctx ).wait().unwrap();
    encodable_mail.encode( &mut encoder )?;

    println!( "{}", encoder.into_string_lossy().unwrap() );

    Ok( () )


}