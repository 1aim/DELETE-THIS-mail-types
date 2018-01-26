extern crate mail_codec;
extern crate futures;
extern crate mime as media_type;

use media_type::{MULTIPART, RELATED};
use futures::Future;

use mail_codec::prelude::*;
use mail_codec::default_impl::SimpleBuilderContext;

fn get_some_resource() -> Resource {
    Resource::from_text("abcd↓efg".into())
}

fn main() {
    _main().unwrap();
}

fn _main() -> Result<()> {
    let mut encoder = Encoder::new( MailType::Ascii );

    let builder_ctx = SimpleBuilderContext::default();

    let media_type = MediaType::new(MULTIPART, RELATED)?;
    let mail = Builder::multipart( media_type )?
        .header( Subject, "that ↓ will be encoded " )?
        .header( From, [ "tim@tom.nixdomain" ])?
        .body( Builder::singlepart( get_some_resource() ).build()? )?
        .body( Builder::singlepart( get_some_resource() ).build()? )?
        .build()?;



    let encodable_mail = mail.into_encodeable_mail( &builder_ctx ).wait().unwrap();
    encodable_mail.encode( &mut encoder )?;

    println!( "{}", encoder.into_string_lossy().unwrap() );

    Ok( () )


}