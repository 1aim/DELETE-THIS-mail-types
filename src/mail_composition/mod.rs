use std::path::PathBuf;
use std::fmt;
use futures::stream::BoxStream;
use ascii::{ AsciiChar, AsciiString };
use mime;

use error::*;
use headers::Header::*;
use types;
use raw_mail::{ SinglepartMime, MultipartMime, SinglepartMail, MailPart };

use self::data::preprocess_data;
use self::ports::{
    Template,
    DataInterface,
    AttachmentOut,
    Stream,
    EmbeddingOut
};
use self::context::{
    MailSendContext
};

pub mod ports;
pub mod context;
mod resource;
mod data;



pub trait ComposeMail: TemplateEngine {
    fn compose_mail<D: DataInterface>
        ( &self,
          context: &Context,
          send_context: MailSendContext,
          data: D,
          template: <Self as TemplateEngine>::TemplateId
        ) -> Result< Stream >
    {
        let mut data = data;
        let from_mailbox = send_context.from;//compose display name => create Address with display name;
        let to_mailbox = send_context.to.display_name_or_else(
            || self.compose_display_name( context, &data ) );

        data.see_from_mailbox( &from_mailbox );
        data.see_to_mailbox( &to_mailbox );

        let core_headers = vec![
            From( types::AddressList::new_with_first( from_mailbox ) ),
            To( types::AddressList::new_with_first( to_mailbox ) ),
            Subject( types::Unstructured::try_from_string( send_context.subject )? )
            //TODO: what else? MessageId? Signature? ... or is it added by relay
        ];

        let ( embeddings, attatchments ) = preprocess_data( context, &mut data );

        let alternate_bodies = self.templates( context, template, data )?;

        if alternate_bodies.len() == 0 {
            return Err( ErrorKind::NeedPlainAndOrHtmlMailBody.into() )
        }

        let mut attachments = Vec::new();
        let mut bodies = Vec::new();
        for body in alternate_bodies {
            bodies.push( create_mail_body(body, &mut attachments )? );
        }
        let has_attachments = attachments.len() > 0;

        let core_mail = MailPart::Multipart {
            mime: gen_multipart_mime( "alternate" ),
            bodies: bodies,
            additional_headers: if has_attachments { Vec::new() } else { core_headers },
            hidden_text: AsciiString::new()
        };

        let mail = if !has_attachments {
            core_mail
        } else {
            MailPart::Multipart {
                mime: gen_multipart_mime( "mixed" ),
                bodies: attachments.map(cre).collect(),
                additional_headers: core_headers,
                hidden_text: AsciiString::new()
            }
        };


        let mut encoder = TokioStremMailEncoder::new();

        //this might have to block on reading from the body stream...
        mail.encode( &mut encoder )?;

        Ok( encoder.into_stream() )

    }
}


fn create_mail_body(tmpl: Template, attatchments: &mut Vec<AttachmentOut> ) -> Result<MailPart> {
    attatchments.extend( tmpl.attachments );

    let mut headers = Vec::new();

    let body_stream = match tmpl.data {
        Stream::Ascii( ascii_stream ) => ascii_stream,
        Stream::NonAscii( stream ) => {
            headers.push( ContentTransferEncoding( types::TransferEncoding::Base64 ) );
            base64_encode_stream( stream )
        }
    };

    let inner_body = MailPart::Body( SinglepartMail {
        mime: SinglepartMime::new( tmpl.mime )?,
        headers: headers,
        source: body_stream
    } );

    if tmpl.embeddings.len() == 0 {
        Ok( inner_body )
    } else {
        let mut bodies = Vec::new();
        for body in tmpl.embeddings {
            bodies.push( create_embedding_body( body )? )
        }
        Ok( MailPart::Multipart {
            mime: gen_multipart_mime( "related" )?,
            bodies: bodies,
            additional_headers: Vec::new(),
            hidden_text: AsciiString::new()
        } )
    }
}


fn create_embedding_body( body: EmbeddingOut ) -> Result<MailPart> {
    //TODO with Content-Transfer-Encoding, and Content-Disposition
}

fn create_attachment_body( body: AttachmentOut ) -> Result<MailPart> {
    //TODO with Content-Transfer-Encoding, and Content-Disposition
}


fn gen_multipart_mime( subtype: &str ) -> Result<MultipartMime> {
    // 1. gen boundary
    // 2. "multipart/" + subtype + "; boundary=\"" boundary + "\""
}


impl<T> ComposeMail for T where T: TemplateEngine { }



