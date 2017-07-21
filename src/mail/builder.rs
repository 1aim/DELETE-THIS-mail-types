use std::ops::Deref;
use std::sync::Arc;
use std::borrow::Cow;
use std::path::Path;

use mime::Mime;
use futures::{ Future };
use futures::future::{ self,  BoxFuture };

use error::*;
use types::TransferEncoding;
use headers::Header;
use codec::transfer_encoding::TransferEncodedBuffer;
use utils::Buffer;

use super::resource::Resource;
use super::{ MailPart, Mail, Headers, Body };
use super::utils::is_multipart_mime;


pub trait Elsewhere {
    //FIXME add mime sniffing and file metadata
    type FileFuture: Future<Item=Vec<u8>, Error=Error>;

    // you can use future::lazy( || stdioloadfile( path ) )
    // as this will always be chained with some post processing (content transfer encoding)
    // and then passed to execute_elsewhere
    fn load_file( &self,  path: &Path ) -> Self::FileFuture;
    fn execute_elsewhere<F: Future>( &self, fut: F) -> BoxFuture<Item=F::Item, Error=F::Error>;
}



//SubBuilder
pub struct SubBuilder<E: Elsewhere>(pub Arc<E>);
pub struct Builder<E: Elsewhere>(pub Arc<E>);

struct BuilderShared<E: Elsewhere> {
    e: Arc<E>,
    headers: Headers,
    is_sub_body: bool
}

impl<E: Elsewhere> BuilderShared<E> {

    fn sub( &self ) -> SubBuilder<E> {
        SubBuilder( self.e.clone() )
    }

    fn set_header( &mut self, header: Header, is_multipart: bool ) -> Result<Option<Header>> {
        //move checks for single/multipart from mail_composition here
        match &header {
            &ContentType( ref mime ) => {
                if is_multipart != is_multipart_mime( mime ) {
                    return Err( ErrorKind::ContentTypeAndBodyIncompatible.into() )
                }
            },
            _ => {}
        }

        let name = header.name().into();

        if self.is_sub_body &&
                !( name.as_str().startswith( "Content-" ) || name.as_str().startswith( "X-" ) ) {
            //TODO warn!( "using non Content-/X- header inside multipart body" )
        }

        Ok( self.headers.insert( name, header ) )
    }

    fn build( self, body: MailPart ) -> Result<Mail> {
        Ok( Mail {
            headers: self.headers,
            is_sub_body: self.is_sub_body,
            body: body
        } )
    }
}

pub struct SinglepartBuilderNoBody<E: Elsewhere> {
    inner: BuilderShared<E>,
}
pub struct SinglepartBuilderWithBody<E: Elsewhere> {
    inner: BuilderShared<E>,
    body: Resource
}
pub struct MultipartBuilderNoBody<E: Elsewhere> {
    inner: BuilderShared<E>,
    hidden_text: Option<String>
}
pub struct MultipartBuilderWithBody<E: Elsewhere> {
    inner: BuilderShared<E>,
    hidden_text: Option<String>,
    //FIXME Vec1Plus
    bodies: Vec<Mail>,
}


impl<E: Elsewhere> Builder<E> {
    #[inline]
    fn new<T: MimeLink>( &self, mime: T) -> T::Builder {
        T::Builder::_new( self.e.clone(), mime.into(), false)
    }
}

impl SubBuilder {
    #[inline]
    fn new<T: MimeLink>( &self, mime: T ) -> T::Builder {
        T::Builder::_new( self.e.clone(), mime.into(), true )
    }
}

impl<E: Elsewhere> SinglepartBuilderNoBody<E> {

    //TODO possible move resource::Stream here as well as to_ascii_stream
    // it can be nicely integrate in this method
    fn set_body( self, body: Resource ) -> SinglepartBuilderWithBody {
        SinglepartBuilderWithBody {
            inner: self.inner,
            body: body
        }
    }

    fn set_header( &mut self, header: Header ) -> Result<Self> {
        self.inner.set_header(header, false )?;
        Ok( self )
    }

}

impl<E: Elsewhere> SinglepartBuilderWithBody<E> {

    fn set_header( &mut self, header: Header ) -> Result<Self> {
        self.inner.set_header(header, false )?;
        Ok( self )
    }

    fn build( self ) -> Result<Mail> {
        use self::Resource::*;

        let body: Body = match self.body {
            Buffer( buffer ) => {
                self.inner.e.execute_elsewhere( future::lazy(
                   move || TransferEncodedBuffer::encode_buffer( buffer, None )
                ) ).into()
            },
            Future( future ) => {
                future.and_then( |buffer|
                    self.inner.e.execute_elsewhere(
                        TransferEncodedBuffer::encode_buffer( buffer, None )
                    )
                ).into()
            },
            File { mime, path, alternate_name } => {
                self.inner.e.execute_elsewhere(
                    self.inner.e.load_file( path ).map( |data| {
                        //TODO add file meta, replacing name with alternate_name (if it is some)
                        let buffer = Buffer::new( mime, data );
                        TransferEncodedBuffer::encode_buffer( buffer, None )
                    })
                ).into()
            }
        };

        self.0.build( MailPart::SingleBody { body } )
    }

}

impl<E: Elsewhere> MultipartBuilderNoBody<E> {

    /// # Example
    /// ```ignore
    ///
    /// Builder(Setup).new( MultipartMime::new( mime )? )
    ///     .add_body( |builder| builder
    ///         .new( Singlepart::new( mime )? )
    ///         .set_body( body1 )
    ///     )?
    ///     .add_body( |builder| builder
    ///         .new( Singlepart::new( mime )? )
    ///         .set_body( body2 )
    ///     )?
    /// ```
    fn add_body<FN>( self, body_fn: FN ) -> Result<Self>
            where FN: FnOnce(SubBuilder<E>) -> Result<Mail>
    {
        Ok( MultipartBuilderWithBody {
            inner: self.inner,
            hidden_text: self.hidden_text,
            bodies: vec![ body_fn( self.inner.sub() )? ],
        } )
    }

    fn set_header( &mut self, header: Header ) -> Result<Self> {
        self.inner.set_header( header, true )?;
        Ok( self )
    }
}

impl<E: Elsewhere> MultipartBuilderWithBody<E> {

    fn add_body<FN>( self, body_fn: FN ) -> Result<Self>
        where FN: FnOnce(SubBuilder<E>) -> Result<Mail>
    {
        self.bodies.push( body_fn( self.inner.sub() )? )
        Ok( self )
    }

    fn set_header( self, header: Header ) -> Result<Self> {
        self.inner.set_header( header, true )?;
        Ok( self )
    }

    fn build( self ) -> Result<Mail> {
        self.inner.build( MailPart::MultipleBodies {
            bodies: self.bodies,
            hidden_text: self.hidden_text.unwrap_or( String::new() ),
        } )
    }
}







pub struct SinglepartMime( Mime );

impl SinglepartMime {
    pub fn new( mime: Mime ) -> Result<Self> {
        if !is_multipart_mime( &mime ) {
            Ok( SinglepartMime( mime ) )
        } else {
            Err( ErrorKind::NotSinglepartMime( mime ).into() )
        }
    }
}

impl Into<Mime> for SinglepartMime {
    fn into( self ) -> Mime {
        self.0
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

impl Into<Mime> for MultipartMime {
    fn into( self ) -> Mime {
        self.0
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

//magic to make Builder::new( SinglePart/Multipart ) -> SinglePart/Multipart work
pub trait _Builder {
    fn _new( mime: Mime, is_sub: bool ) -> Self;
}

pub trait MimeLink: Into<Mime> {
    type Builder: _Builder;
}

impl MimeLink for SinglepartMime {
    type Builder = SinglepartBuilderNoBody;
}

impl MimeLink for MultipartMime {
    type Builder = MultipartBuilderNoBody;
}

impl _Builder for SinglepartBuilderNoBody {

    fn _new( content_type: Mime, is_sub: bool ) -> Self {
        assert!( !is_multipart_mime( &content_type ) );
        SinglepartBuilder( BuilderShared {
            headers: new_headers( content_type ),
            is_sub_body: is_sub
        } )
    }
}

impl _Builder for MultipartBuilderNoBody {
    fn _new( content_type: Mime, is_sub: bool ) -> Self {
        assert!( is_multipart_mime( &content_type ) );
        MultipartBuilderNoBody{
            inner: BuilderShared {
                headers: new_headers( content_type ),
                is_sub_body: is_sub
            },
            hidden_text: None
        }
    }
}

fn new_headers( content_type: Mime ) -> Headers {
    let mut headers = Headers::new();
    let header = Header::ContentType( mime );
    headers.insert( header.name().into(), header );
    headers
}
