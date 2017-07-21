
//FIXME possible merge with ressource
#[derive(Debug)]
pub struct Body {
    transfer_encoding: TransferEncoding,
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
    pub fn new( unencoded: BufferFuture, transfer_encoding: TransferEncoding ) -> Result<Body> {
        let tencode_fn = TRANSFER_ENCODINGS.lookup( &transfer_encoding )?;
        //for now tencoder is a function, if not make it
        //move |buffer| { tencoder.encode( ... ) }
        let res = unencoded.and_then( tencode_fn );
        Body {
            transfer_encoding: transfer_encoding,
            body: InnerBody::Future( res )
        }
    }

    //WHEN_FEATURE(fast_ascii_mails)
    //is a optimization postpone it for now
    //fn new_just_ascii(...)

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

