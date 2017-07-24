use std::result::{ Result as StdResult };
use std::ops::Deref;
//FIXME use FnvHashMap
use std::collections::HashMap;
use std::borrow::Cow;

use mime::{ Mime, MULTIPART, BOUNDARY };
use ascii::{ AsciiString, AsciiChar };
use futures::future::{ BoxFuture, ok as future_ok };
use futures::{ Future, Async, Poll, IntoFuture };

use codec::transfer_encoding::TRANSFER_ENCODINGS;
use types::TransferEncoding;
use headers::Header;
use error::*;
use utils::{ BufferFuture, Buffer };

use self::body::Body;
pub use self::builder::*;

mod utils;
pub mod body;
pub mod resource;
pub mod mime;
mod builder;
mod encode;


type Headers = HashMap<Cow<'static, AsciiStr>, Header>;


pub struct Mail {
    //NOTE: by using some OwnedOrStaticRef AsciiStr we can probably safe a lot of
    // unnecessary allocations
    headers: Headers,
    body: MailPart,
}


pub enum MailPart {
    SingleBody {
        // a future stream of bytes?
        // we apply content transfer encoding on it but no dot-staching as that
        // is done by the protocol, through its part of the mail so it would be
        // interesting to dispable dot staching on protocol level as we might
        // have to implement support for it in this lib for non smtp mail transfer
        // also CHUNKED does not use dot-staching making it impossible to use it
        // with tokio-smtp
        body: Body
    },
    MultipleBodies {
        bodies: Vec<Mail>,
        hidden_text: AsciiString
    }
}

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
    pub fn set_header( &self, header: Header ) -> Result<Option<Header>> {
        use headers::Header::*;

        match &header {
            &ContentType( ref mime ) => {
                if self.body.is_multipart() != is_multipart_mime( mime ) {
                    return Err( ErrorKind::ContentTypeAndBodyIncompatible.into() )
                }
            },
            ContentTransferEncoding( ref encoding ) => {
                //TODO warn as this is most likly leading to unexpected results
            },
            _ => {}
        }

        Ok( self.headers.insert( header.name().into(), header ) )

    }

    pub fn headers( &self ) -> &[Header] {
        &*self.headers
    }

    pub fn body( &self ) -> &MailPart {
        &self.body
    }

    fn walk_mail_bodies_mut<FN>( &mut self, use_it_fn: FN)
        where FN: FnMut( &mut Body )
    {
        use self::MailPart::*;
        match self.body {
            SingleBody { ref mut body } =>
                use_it_fn( body ),
            MultipleBodies { ref mut bodies, .. } =>
                for body in bodies {
                    body.walk_mail_bodies_mut( use_it_fn )
                }
        }
    }
}

impl IntoFuture for Mail {
    type Future = MailFuture;
    type Item = Mail;
    type Error = Error;

    /// converts the Mail into a future,
    ///
    /// the future resolves once
    /// all contained BodyFutures are resolved (or one of
    /// them resolves into an error in which case it will
    /// resolve to the error and cancel all other BodyFutures)
    ///
    ///
    fn into_future(self) -> Self::Future {
        MailFuture( self )
    }
}


pub struct MailFuture( Option<Mail> );

impl MailPart {

    pub fn is_multipart( &self ) -> bool {
        use self::MailPart::*;
        match *self {
            SingleBody { .. } => false,
            MultipleBodies { .. } => true
        }
    }
}


impl Future for MailFuture {
    type Item = Mail;
    type Error = Error;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        let mut done = true;
        self.0.as_mut()
            // this is conform with how futures work, as calling poll on a random future
            // after it completes has unpredictable results (through one of NotRady/Err/Panic)
            // use `Fuse` if you want more preditable behaviour in this edge case
            .expect( "poll not to be called after completion" )
            .walk_mail_bodies_mut( |body| {
                match body.poll_body() {
                    Ok( None ) =>
                        done = false,
                    Err( err ) => {
                        return Err( err )
                    }
                }
            });

        if done {
            Ok( Async::Ready( self.0.take().unwrap() ) )
        } else {
            Ok( Async::NotReady )
        }
    }
}