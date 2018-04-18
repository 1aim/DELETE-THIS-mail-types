use soft_ascii_string::{
    SoftAsciiStr,
    SoftAsciiChar,
    SoftAsciiString
};
use media_type::BOUNDARY;

use core::error::{Result, ErrorKind, Error};
use core::encoder::{Encoder, EncodableInHeader, EncodeHandle};
use core::header::{HeaderName};
use mheaders::ContentType;

use super::{
    Mail, EncodableMail,
    Resource,
};




///
/// # Panics
/// if the body is not yet resolved use `Body::poll_body` or `IntoFuture`
/// on `Mail` to prevent this from happening
///
#[inline(always)]
pub fn encode_mail(
    mail: &EncodableMail,
    top: bool,
    encoder: &mut Encoder<Resource>
) -> Result<()> {
    _encode_mail( &mail.0, top, encoder )
}

fn _encode_mail(
    mail: &Mail,
    top: bool,
    encoder: &mut Encoder<Resource>
) -> Result<()> {
    encode_headers( &mail, top, encoder )?;

    //the empty line between the headers and the body
    encoder.add_blank_line();

    encode_mail_part( &mail, encoder )?;

    Ok( () )
}

///
/// # Panics
/// if the body is not yet resolved use `Body::poll_body` or `IntoFuture`
/// on `Mail` to prevent this from happening
///
fn encode_headers(
    mail: &Mail,
    top: bool,
    encoder:  &mut Encoder<Resource>
) -> Result<()> {
    let mut handle = encoder.encode_handle();
    if top {
        handle.write_str(SoftAsciiStr::from_str_unchecked(
            "MIME-Version: 1.0"
        ))?;
        handle.finish_header();
    }

    for (name, hbody) in mail.headers.iter() {
        let name_as_str = name.as_str();
        let ignored_header = !top &&
            !(name_as_str.starts_with("Content-")
                || name_as_str.starts_with("X-") );

        if ignored_header {
            warn!("non `Content-` header in MIME body: {:?}: {:?}", name, hbody);
        }

        encode_header( &mut handle, name, hbody)?;
    }
    Ok( () )
}

fn encode_header(
    handle: &mut EncodeHandle,
    name: HeaderName,
    component: &EncodableInHeader
) -> Result<()> {
    handle.write_str( name.as_ascii_str() )?;
    handle.write_char( SoftAsciiChar::from_char_unchecked(':') )?;
    handle.write_fws();
    component.encode( handle )?;
    handle.finish_header();
    Ok( () )
}

///
/// # Panics
/// if the body is not yet resolved use `Body::poll_body` or `IntoFuture`
/// on `Mail` to prevent this from happening
///
fn encode_mail_part(mail: &Mail, encoder:  &mut Encoder<Resource> ) -> Result<()> {
    let minus = SoftAsciiChar::from_char_unchecked('-');

    use super::MailPart::*;
    match mail.body {
        SingleBody { ref body } => {
            //Note: Resource is a Arc so sheap to clone
            encoder.add_body(body.clone());
        },
        MultipleBodies { ref hidden_text, ref bodies } => {
            if hidden_text.len() > 0 {
                warn!("hidden_text fields in multipart bodies are currently not encoded")
            }
            let boundary;
            if let Some( media_type ) = mail.headers.get_single(ContentType) {
                boundary = media_type?.get_param(BOUNDARY)
                    //TODO typed error
                    .ok_or_else( ||-> Error { "[BUG] boundary gone missing".into() } )?
                    .to_content()
            } else {
                bail!( "Content-Type header gone missing" )
            }


            let boundary = SoftAsciiString::from_string(boundary)
                .map_err( |_orig_string| {
                    ErrorKind::Msg("non ascii boundary".into())
                })?;

            for mail in bodies.iter() {
                encoder.write_header_line(|handle| {
                    handle.write_char( minus )?;
                    handle.write_char( minus )?;
                    handle.write_str( &*boundary )
                })?;
                _encode_mail( mail, false, encoder )?;
            }

            if bodies.len() > 0 {
                encoder.write_header_line(|handle| {
                    handle.write_char( minus )?;
                    handle.write_char( minus )?;
                    handle.write_str( &*boundary )?;
                    handle.write_char( minus )?;
                    handle.write_char( minus )
                })?;
            } else {
                warn!("multipart body with 0 sub bodies")
            }

        }
    }
    Ok( () )
}
