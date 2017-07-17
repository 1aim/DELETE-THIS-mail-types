use std::result::{ Result as StdResult };
use std::ops::Deref;

use mime::{ Mime, MULTIPART, BOUNDARY };
use ascii::AsciiString;
use futures::stream::BoxStream;

use headers::Header;
use error::*;
// structure with mime multipart etc.
// possible with futures



pub enum MailPart {
    Body( SinglepartMail ),
    Multipart {
        mime: MultipartMime,
        bodies: Vec<MailPart>,
        // there can be more headers then "just" Content-Type: multipart/xxx
        // Through I don't know if it makes sense in any position except the outer most MimeBody
        additional_headers: Vec<Header>,
        // there is usable (plain text) space between the headers and the first sub body
        // it can be used mainly for email clients not supporting  multipart mime
        hidden_text: AsciiString,
    }
}

// if true this is a multi part body
// as such headers _SHOULD_ only contain `Content-` Headers (others are ignored)
// if false its a stand alone email
pub struct SinglepartMail {
    mime: SinglepartMime,
    // is not allowed to contain Content-Type, as this is given through the mime field
    // if Content-Transfer-Encoding is present it's used as PREFFERED encoding, the
    // default and fallback (e.g. 8bit but no support fo 8bit) is Base64
    headers: Vec<Header>,
    // a future stream of bytes?
    // we apply content transfer encoding on it but no dot-staching as that
    // is done by the protocol, through its part of the mail so it would be
    // interesting to dispable dot staching on protocol level as we might
    // have to implement support for it in this lib for non smtp mail transfer
    // also CHUNKED does not use dot-staching making it impossible to use it
    // with tokio-smtp
    source: BoxStream<Item=u8, Error=Error>
}



pub struct SinglepartMime( Mime );

impl SinglepartMime {
    pub fn new( mime: Mime ) -> Result<Self> {
        if mime.type_() != MULTIPART {
            Ok( SinglepartMime( mime ) )
        } else {
            Err( ErrorKind::NotSinglepartMime( mime ).into() )
        }
    }
}

impl Deref for SinglepartMime {
    type Target = Mime;

    fn deref( &self ) -> &Mime {
        &self.0
    }
}

pub struct MultipartMime( Mime );

impl MultipartMime {

    pub fn new( mime: Mime ) -> Result<Self> {
        if mime.type_() == MULTIPART {
            check_boundary( &mime )?;
            Ok( MultipartMime( mime ) )
        }  else {
            Err( ErrorKind::NotMultipartMime( mime ).into() )
        }

    }
}

impl Deref for MultipartMime {
    type Target = Mime;

    fn deref( &self ) -> &Mime {
        &self.0
    }
}

fn check_boundary( mime: &Mime ) -> Result<()> {
    mime.get_param( BOUNDARY )
        .map( |_|() )
        .ok_or_else( || ErrorKind::MultipartBoundaryMissing.into() )
}