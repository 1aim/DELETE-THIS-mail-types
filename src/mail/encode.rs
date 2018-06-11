use failure::Fail;
use soft_ascii_string::{
    SoftAsciiStr,
    SoftAsciiChar,
    SoftAsciiString
};
use media_type::BOUNDARY;

use common::error::{EncodingError, EncodingErrorKind, Place, UTF_8, US_ASCII};
use common::encoder::{
    EncodingBuffer, EncodableInHeader, EncodingWriter,
};
use headers::{HeaderName, ContentType};
use headers::error::{HeaderValidationError, BuildInValidationError};
use ::error::MailError;

use super::{
    Mail, EncodableMail
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
    encoder: &mut EncodingBuffer
) -> Result<(), MailError> {
    _encode_mail(&mail.0, top, encoder)
        .map_err(|err| {
            let mail_type = encoder.mail_type();
            use self::MailError::*;

            match err {
                 Encoding(enc_err) => Encoding(enc_err.with_mail_type_or_else(||Some(mail_type))),
                 other => other
            }
        })
}

fn _encode_mail(
    mail: &Mail,
    top: bool,
    encoder: &mut EncodingBuffer
) -> Result<(), MailError> {
    encode_headers(&mail, top, encoder)?;

    //the empty line between the headers and the body
    encoder.write_blank_line();

    encode_mail_part(&mail, encoder)?;

    Ok(())
}

///
/// # Panics
/// if the body is not yet resolved use `Body::poll_body` or `IntoFuture`
/// on `Mail` to prevent this from happening
///
fn encode_headers(
    mail: &Mail,
    top: bool,
    encoder:  &mut EncodingBuffer
) -> Result<(), MailError> {
    let mut handle = encoder.writer();
    if top {
        handle.write_str(SoftAsciiStr::from_unchecked(
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

        encode_header(&mut handle, name, hbody)
            .map_err(|err| {
                err.with_place_or_else(|| Some(Place::Header { name: name.as_str() }))
            })?;
    }
    Ok( () )
}

fn encode_header(
    handle: &mut EncodingWriter,
    name: HeaderName,
    component: &EncodableInHeader
) -> Result<(), EncodingError> {
    handle.write_str(name.as_ascii_str())?;
    handle.write_char(SoftAsciiChar::from_unchecked(':'))?;
    handle.write_fws();
    component.encode(handle)?;
    handle.finish_header();
    Ok(())
}

///
/// # Panics
/// if the body is not yet resolved use `Body::poll_body` or `IntoFuture`
/// on `Mail` to prevent this from happening
///
fn encode_mail_part(mail: &Mail, encoder:  &mut EncodingBuffer )
    -> Result<(), MailError>
{
    let minus = SoftAsciiChar::from_unchecked('-');

    use super::MailPart::*;
    match mail.body {
        SingleBody { ref body } => {
            encoder.write_body_unchecked(body)?;
        },
        MultipleBodies { ref hidden_text, ref bodies } => {
            if hidden_text.len() > 0 {
                //TODO find out if there is any source using the hidden text
                // (e.g. for some form of validation, prove of senders validity etc.)
                // if not drop the "hidden_text" field
                warn!("\"hidden text\" in multipart bodies is dropped")
            }
            let boundary;
            if let Some(media_type) = mail.headers.get_single(ContentType) {
                let media_type = media_type.map_err(|err|
                    HeaderValidationError
                    ::from(err.context(BuildInValidationError::NoContentType))
                )?;

                boundary = media_type.get_param(BOUNDARY)
                    //TODO typed error
                    .ok_or_else(|| HeaderValidationError
                        ::from(BuildInValidationError::NoMultipartBoundary)
                    )?
                    .to_content()
            } else {
                return Err(HeaderValidationError::from(
                    BuildInValidationError::NoContentType).into());
            }


            let boundary = SoftAsciiString
                ::from_string(boundary)
                .map_err(|orig_string| EncodingError
                    ::from(EncodingErrorKind::InvalidTextEncoding {
                        got_encoding: UTF_8,
                        expected_encoding: US_ASCII
                    })
                    .with_place_or_else(|| Some(Place::Header { name: "Content-Type" }))
                    .with_str_context(orig_string.into_source())
                )?;

            for mail in bodies.iter() {
                encoder.write_header_line(|handle| {
                    handle.write_char(minus)?;
                    handle.write_char(minus)?;
                    handle.write_str(&*boundary)
                })?;
                _encode_mail(mail, false, encoder)?;
            }

            if bodies.len() > 0 {
                encoder.write_header_line(|handle| {
                    handle.write_char(minus)?;
                    handle.write_char(minus)?;
                    handle.write_str(&*boundary)?;
                    handle.write_char(minus)?;
                    handle.write_char(minus)
                })?;
            } else {
                // this is theoretically not necessary an error, but it
                // really should not be created, the only use I could come
                // up with is to e.g. hide additional data "between the lines"
                // in a mail used to control some malware
                return Err(HeaderValidationError::from(
                    BuildInValidationError::EmptyMultipartBody).into());
            }

        }
    }
    Ok(())
}
