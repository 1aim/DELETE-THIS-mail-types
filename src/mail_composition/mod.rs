use std::path::PathBuf;
use std::fmt;
use futures::stream::BoxStream;
use ascii::{ AsciiChar, AsciiString };
use mime;

use error::*;
use raw_mail::{ SinglepartMime, MultipartMime, SinglepartMail, MailPart };

pub mod context;
mod resource;
mod data;
pub mod ports;



pub trait ComposeMail: TemplateEngine {
    fn compose_mail( &self,
                     context: &Context,
                     send_context: MailSendContext,
                     data: Data,
                     template: <Self as TemplateEngine>::TemplateId
        ) -> Result< Stream >
    {
        let data = data.with_from( from ).with_to( to );
        let (data, embeddings, attatchments) = data.preprocess_data()?;
        let from_mailbox = send_context.from;//compose display name => create Address with display name;
        let to_mailbox = send_context.to.display_name_or_else(
            || self.compose_display_name( context, &data ) );

        let alternate_bodies = self.templates( context, template, data )?;

        if !( plain_body.is_some() || html_body.is_some() ) {
            return Err( ErrorKind::NeedPlainAndOrHtmlMailBody.into() )
        }
        let mut bodies = Vec::new();
        if let Some( plain_body ) = plain_body {
            bodies.push( MailPart::Body( SinglepartMail {
                mime: mime::TEXT_PLAIN,
                headers: vec![

                ],
                source: plain_body,
            } ) )
        }
        if let Some( html_body ) = html_body {
            bodies.push( MailPart::Multipart {
                mime: gen_multipart_mime("related")?,
                bodies: vec![
                    MailPart::Body( SinglepartMail {
                        mime: SinglepartMime::new( mime::TEXT_HTML )?,
                        headers: vec![

                        ],
                        source: html_body,
                    })
                    //... embedings
                ],
                additional_headers: Vec::new(),
                hidden_text: AsciiString::new(),
            } )
        }
        let mail = MailPart::Multipart {
            mime: gen_multipart_mime("mixed")?,
            bodies: bodies,
            additional_headers: Vec::new(),
            hidden_text: AsciiString::new(),
        };

        let mut encoder = TokioStremMailEncoder::new();

        //this might have to block on reading from the body stream...
        mail.encode( &mut encoder )?;

        Ok( encoder.into_stream() )

    }
}

fn gen_multipart_mime( subtype: &str ) -> Result<MultipartMime> {

}


impl<T> ComposeMail for T where T: TemplateEngine { }



