use mime::Mime;
use ascii::AsciiString;

use headers::Header;

// structure with mime multipart etc.
// possible with futures

pub struct Boundary( String );
pub struct MultipartMIME( MIME );
pub struct SinglepartMIME( MIME );



pub enum Mail {
    Body( SinglepartMail ),
    Multipart {
        mime: MultipartMIME,
        boundary: Boundary,
        bodies: Vec<MimeBody>,
        // there can be more headers then "just" Content-Type: multipart/xxx
        // Through I don't know if it makes sense in any position except the outer most MimeBody
        additional_headers: Vec<Header>,
        // there is usable (plain text) space between the headers and the first sub body
        // it can be used mainly for email clients not supporting  multipart mime
        hidden_text: AsciiString,
    }
}

pub struct SinglepartMail {
    mime: SinglepartMIME,
    // if true this is a multi part body
    // as such headers _SHOULD_ only contain `Content-` Headers (others are ignored)
    // if false its a stand alone email
    is_multipart_body: bool,
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
    source: TODO
}

