extern crate mail_codec as mail;
extern crate futures;
extern crate mime as media_type;
extern crate futures_cpupool;

use media_type::{MULTIPART, RELATED};
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
    let mut encoder = Encoder::new( MailType::Ascii );

    let resource_loader: FsResourceLoader = FsResourceLoader::with_cwd_root().unwrap();
    let builder_ctx = CompositeBuilderContext::new(
        resource_loader,
        CpuPoolBuilder::new().create()
    );

    let media_type = MediaType::new(MULTIPART, RELATED)?;
    let mail = Builder::multipart( media_type )?
        .header( Subject, "that ↓ will be encoded " )?
        .header( From, [ "tim@tom.nixdomain" ])?
        .body( Builder::singlepart( get_some_resource() ).build()? )?
        .body( Builder::singlepart( get_some_resource() ).build()? )?
        .build()?;



    let encodable_mail = mail.into_encodeable_mail( &builder_ctx ).wait().unwrap();
    encodable_mail.encode( &mut encoder )?;

    println!( "{}", encoder.to_string_lossy().unwrap() );

    Ok( () )


}