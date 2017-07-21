use mime::Mime;
use futures::Either;

use utils::{ Buffer, MimeBitDomain };
use types::TransferEncoding;

//FIXME possible merge with ressource
#[derive(Debug)]
pub struct Body {
    body: InnerBody
}

enum InnerBody {
    /// a futures resolving to a buffer
    Future(BufferFuture),
    /// store the value the BufferFuture resolved to
    Value(Buffer),
    /// if the BufferFuture failed, we don't have anything
    /// to store, but have not jet dropped the mail it is
    /// contained within, so we still need a value for InnerBody
    ///
    /// this variations should only ever occure between
    /// a call to a BodyFuture in `MailFuture::poll` resolved to
    /// an Error and the Body/Mail being dropped (before `MailFuture::poll`
    /// exists)
    Failed
}


impl Body {

    /// creates a new body based on the `data` Buffer,
    ///
    /// if data is not yet transfer encoded:
    /// - it will be encoded with transfer_encoding if it is some
    /// - or if it 7bit it won't be encode (but checked)
    /// - or it will be encoded with QuotedPrintable if it is text
    /// - or it will be encoded with base64 if it isn't text
    ///
    /// if data is already transfer encoded and `transfer_encoding` is
    /// `transfer_encoding` will be ignores, but a warning is triggered
    pub fn new( data: BufferFuture, transfer_encoding: Option<TransferEncoding> ) -> Result<Body> {
        let preset =
            match transfer_encoding.map( |encoding| TRANSFER_ENCODINGS.lookup( encoding ) ) {
                Some( result ) => Some( result? ),
                None => None
            };
        
        let body_future = data.and_then( move |buffer: Buffer| {
             if buffer.content_transfer_encoding().is_some() {
                 //TODO if preset.is_some() { warn!() }
                 return future::ok( buffer ).boxed();
             }
             let func = preset.unwrap_or_else(|| {
                 let encoding =
                     if buffer.bit_domain == MimeBitDomaim::_7Bit {
                         TranserEncoding::_7Bit
                     } else if buffer.is_text() {
                         TransferEncoding::QuotedPrintable
                     } else {
                         TransferEncoding::Base64
                     };
                TRANSFER_ENCODINGS.lookup( encoding )
                    .expect( "_7Bit/quoted_printable/base64 are preset and therefore can not be not available" );
            });

            func( buffer )

        }).boxed();

        Body {
            body: InnerBody::Future( body_future )
        }
    }

    /// returns a reference to the buffer if
    /// the buffer is directly contained in the Body,
    /// i.e. the Futures was resolved _and_ the body
    /// is aware of it
    ///
    pub fn buffer_ref( &self ) -> Option<&Buffer> {
        use self::InnerBody::*;
        match self.body {
            Value( ref value ) => Some( value ),
            _ => None
        }
    }

    /// polls the body for completation by calling `Futures::poll` on the
    /// contained future
    ///
    /// returns:
    /// - Ok(Some),  if the future was already completed in the past
    /// - Ok(Some),* if polll results in Ready, also the contained future
    ///              will be replaced by the value it resolved to
    /// - Ok(None),  if the future is not ready yet
    /// - Err(),     if the future resolved to a err in a previous call to
    ///              poll_body, note that the error the future resolved to
    ///              is no longer available
    /// - Err(),*    if the future resolves to an Error, the contained future
    ///              will be removed, `chain_err` will be used to include
    ///              the error in the error_chain
    pub fn poll_body( &mut self ) -> Result<Option<&Buffer>> {
        use self::InnerBody::*;
        let mut new_body = None;
        match self.body {
            Failed =>
                return Err( ErrorKind::BodyFutureResolvedToAnError.into() )
            Value( ref buffer ) =>
                return Ok( Some( buffer ) ),
            Future( ref mut future ) => {
                match future.poll() {
                    Ok( Async::NotReady ) => {},
                    Ok( Async::Ready( buffer ) ) =>
                        new_body = Ok( Some( buffer ) ),
                    Err( e ) =>
                        new_body = Err( e )
                }
            },
        }

        match new_body {
            Ok( None ) => Ok( None ),
            Ok( Some( buffer ) ) => {
                self.body = Value( buffer );
                self.buffer_ref()
            }
            Err( e ) => {
                self.body = Failed;
                Err( e )
            }
        }
    }
}

