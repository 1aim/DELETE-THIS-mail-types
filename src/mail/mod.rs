use std::ops::Deref;
use std::fmt;

use codec::{EncodableInHeader, Encoder, Encodable, EncodeHandle};
use ascii::{ AsciiString, AsciiChar };
use futures::{ Future, Async, Poll };

use error::*;
use utils::HeaderTryInto;
use headers::{
    Header, HeaderMap,
    ContentType, From,
    ContentTransferEncoding,
    Date, MessageId
};
use components::DateTime;

use self::builder::{
    check_header,
    check_multiple_headers,
};

pub use self::builder::{
    Builder, MultipartBuilder, SinglepartBuilder
};
pub use self::context::*;
pub use self::resource::*;

pub mod mime;
mod resource;
mod builder;
mod encode;
mod context;



pub struct Mail {
    //NOTE: by using some OwnedOrStaticRef AsciiStr we can probably safe a lot of
    // unnecessary allocations
    headers: HeaderMap,
    body: MailPart,
}


pub enum MailPart {
    SingleBody {
        body: Resource
    },
    MultipleBodies {
        bodies: Vec<Mail>,
        hidden_text: AsciiString
    }
}

/// a future resolving to an encodeable mail
///
/// The future resolves like this:
/// 1. it makes sure all contained futures are resolved, i.e. all
///    `Resources` are loaded and transfer encoded if needed
/// 2. it inserts auto-generated headers, i.e. `Content-Type`,
///    `Content-Transfer-Encoding` are generated and `Date`, too if
///    it is needed.
/// 3. contextual validators are used (including a check if there is
///    a `From` header)
/// 4. as the mail is now ready to be encoded it resolves to an
///    `EncodableMail`
///
/// # Error (while resolving the future)
///
/// - if one of the contained futures fails, e.g. if a resource can not
///   be loaded or encoded
/// - if a contextual validator fails, e.g. `From` header is missing or
///   there is a multi mailbox `From` header but no `Sender` header
///
pub struct MailFuture<'a, T: 'a> {
    mail: Option<Mail>,
    ctx: &'a T
}

/// a mail with all contained futures resolved, so that it can be encoded
pub struct EncodableMail( Mail );

impl Mail {


    /// adds a new header,
    ///
    /// - if the header already existed, the existing one will be overriden and the
    ///   old header will be returned
    /// - `Content-Transfer-Encoding` it might be overwritten later one
    ///
    /// # Failure
    ///
    /// if a Content-Type header is set, which conflicts with the body, mainly if
    /// you set a multipart content type on a non-multipart body or the other way around
    ///
    pub fn set_header<H, C>( &mut self, header: H, comp: C) -> Result<()>
        where H: Header,
              H::Component: EncodableInHeader,
              C: HeaderTryInto<H::Component>
    {
        let comp = comp.try_into()?;
        check_header::<H>( &comp, self.body.is_multipart() )?;
        self.headers.insert( header, comp )?;
        Ok( () )
    }

    pub fn set_headers( &mut self, headers: HeaderMap ) -> Result<()> {
        check_multiple_headers( &headers, self.body.is_multipart() )?;
        self.headers.extend( headers )?;
        Ok( () )
    }

    pub fn headers( &self ) -> &HeaderMap {
        &self.headers
    }

    pub fn body( &self ) -> &MailPart {
        &self.body
    }

    /// Turns the mail into a future with resolves to an `EncodeableMail`
    ///
    pub fn into_encodeable_mail<'a, C: BuilderContext>(self, ctx: &'a C ) -> MailFuture<'a, C> {
        MailFuture {
            ctx,
            mail: Some( self )
        }
    }

    fn walk_mail_bodies_mut<FN>( &mut self, use_it_fn: &mut FN) -> Result<()>
        where FN: FnMut( &mut Resource ) -> Result<()>
    {
        use self::MailPart::*;
        match self.body {
            SingleBody { ref mut body } =>
                use_it_fn( body )?,
            MultipleBodies { ref mut bodies, .. } =>
                for body in bodies {
                    body.walk_mail_bodies_mut( use_it_fn )?
                }
        }
        Ok( () )
    }
}







impl MailPart {

    pub fn is_multipart( &self ) -> bool {
        use self::MailPart::*;
        match *self {
            SingleBody { .. } => false,
            MultipleBodies { .. } => true
        }
    }
}


impl<'a, T> Future for MailFuture<'a, T>
    where T: BuilderContext,
{
    type Item = EncodableMail;
    type Error = Error;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        let mut done = true;
        let ctx: &T = &self.ctx;
        self.mail.as_mut()
            // this is conform with how futures work, as calling poll on a random future
            // after it completes has unpredictable results (through one of NotReady/Err/Panic)
            // use `Fuse` if you want more preditable behaviour in this edge case
            .expect( "poll not to be called after completion" )
            .walk_mail_bodies_mut( &mut |body: &mut Resource| {
                match body.poll_encoding_completion( ctx ) {
                    Ok( Async::NotReady ) => {
                        done = false;
                        Ok(())
                    },
                    Ok( Async::Ready( .. ) ) => {
                        Ok(())
                    },
                    Err( err ) => {
                         Err( err )
                    }
                }
            })?;

        if done {
            EncodableMail::from_loaded_mail( self.mail.take().unwrap() )
                .map( |enc_mail| Async::Ready(enc_mail) )
        } else {
            Ok( Async::NotReady )
        }
    }
}

impl EncodableMail {

    fn from_loaded_mail(mut mail: Mail) -> Result<Self> {
        insert_generated_headers(&mut mail)?;
        // also insert `Date` if needed, but only on the outer most header map
        if !mail.headers.contains(Date) {
            mail.headers.insert(Date, DateTime::now())?;
        }

        mail.headers.use_contextual_validators()?;

        // also check `From` only on the outer most header map
        if !mail.headers.contains(From) {
            bail!("mail must have a `From` header");
        }
        if !mail.headers.contains(MessageId) {
            //warn "mail should have a MessageId
        }

        Ok(EncodableMail(mail))
    }
}

/// inserts ContentType and ContentTransferEncoding into
/// the headers of any contained `MailPart::SingleBody`,
/// based on the `Resource` representing the body
fn insert_generated_headers(mail: &mut Mail) -> Result<()> {
    match mail.body {
        MailPart::SingleBody { ref body } => {
            let file_buffer = body.get_if_encoded()?
                .expect("encoded mail, should only contain already transferencoded resources");

            mail.headers.insert(ContentType, file_buffer.content_type().clone())?;
            mail.headers.insert(ContentTransferEncoding, file_buffer.transfer_encoding().clone())?;
        }
        MailPart::MultipleBodies { ref mut bodies, .. } => {
            for sub_mail in bodies {
                insert_generated_headers(sub_mail)?;
            }
        }

    }
    Ok(())
}

impl Deref for EncodableMail {

    type Target = Mail;
    fn deref( &self ) -> &Self::Target {
        &self.0
    }
}

impl Into<Mail> for EncodableMail {
    fn into( self ) -> Mail {
        self.0
    }
}

impl Encodable<Resource> for EncodableMail {

    fn encode(&self, encoder:  &mut Encoder<Resource>) -> Result<()> {
        // does not panic as a EncodableMail only is constructed from
        // a Mail which has all of it's bodies resolved, without failure
        encode::encode_mail( &self, true, encoder )
    }
}

impl fmt::Debug for EncodableMail {
    fn fmt(&self, fter: &mut fmt::Formatter) -> fmt::Result {
        write!(fter, "EncodableMail {{ .. }}")
    }
}