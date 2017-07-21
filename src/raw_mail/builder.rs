use std::ops::Deref;
use std::borrow::Cow;

use mime::Mime;

use error::*;
use types::TransferEncoding;
use headers::Header;
use super::body::Stream;
use super::{ MailPart, Mail, Headers, Body };
use super::utils::is_multipart_mime;

const DEFAULT_TRANSFER_ENCODING: &TransferEncoding = &TransferEncoding::Base64;


//SubBuilder
pub struct SubBuilder;
pub struct Builder;

struct BuilderShared {
    headers: Headers,
    is_sub_body: bool
}

impl BuilderShared {

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

pub struct SinglepartBuilderNoBody( BuilderShared );
pub struct SinglepartBuilderWithBody( BuilderShared, Stream );
pub struct MultipartBuilderNoBody{
    inner: BuilderShared,
    hidden_text: Option<String> );
}
pub struct MultipartBuilderWithBody {
    inner: BuilderShared,
    hidden_text: Option<String>,
    //FIXME Vec1Plus
    bodies: Vec<Mail>,
}


impl Builder {
    #[inline]
    fn new<T: MimeLink>(mime: T) -> T::Builder {
        T::Builder::_new(mime.into(), false)
    }
}

impl SubBuilder {
    #[inline]
    fn new<T: MimeLink>( mime: T ) -> T::Builder {
        T::Builder::_new( mime.into(), true )
    }
}

impl SinglepartBuilderNoBody {

    //TODO possible move resource::Stream here as well as to_ascii_stream
    // it can be nicely integrate in this method
    fn set_body( self, body: Stream ) -> SinglepartBuilderWithBody {
        SinglepartBuilderWithBody( self.0, body )
    }

    fn set_header( &mut self, header: Header ) -> Result<Self> {
        self.0.set_header(header, false )?;
        Ok( self )
    }

}

impl SinglepartBuilderWithBody {

    fn set_header( &mut self, header: Header ) -> Result<Self> {
        self.0.set_header(header, false )?;
        Ok( self )
    }

    fn build( self ) -> Result<Mail> {
        let transfer_encoding = self.0.headers.get( Cow::Borrowed(
            ascii_str!{ C o n t e n t Minus T r a n s f e r Minus E n c o d i n g }
        ) );

        let body = match self.1 {
            Stream::Ascii( ascii_stream ) => {
                if let Some( encoding ) = transfer_encoding {
                    Body::new( ascii_stream.map( |ascii_char| ascii_char as u8 ), encoding )?
                } else {
                    Body::new_just_ascii( ascii_stream )
                }
            },
            Stream::NonAscii( stream ) => {
                let encoding = transfer_encoding.unwrap_or( DEFAULT_TRANSFER_ENCODING );
                Body::new( stream, encoding )?
            }
        };


        self.0.build( MailPart::SingleBody { body } )
    }

}

impl MultipartBuilderNoBody {

    /// # Example
    /// ```ignore
    ///
    /// Builder::new( MultipartMime::new( mime )? )
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
            where FN: FnOnce(SubBuilder) -> Result<Mail>
    {
        Ok( MultipartBuilderWithBody {
            inner: self.inner,
            hidden_text: self.hidden_text,
            bodies: vec![ body_fn( SubBuilder )? ],
        } )
    }

    fn set_header( &mut self, header: Header ) -> Result<Self> {
        self.inner.set_header( header, true )?;
        Ok( self )
    }
}

impl MultipartBuilderWithBody {

    fn add_body<FN>( self, body_fn: FN ) -> Result<Self>
        where FN: FnOnce(SubBuilder) -> Result<Mail>
    {
        self.bodies.push( body_fn( SubBuilder )? )
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
