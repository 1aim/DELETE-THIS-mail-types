//TODO
//  we don't know all exact mimes when we use the builder
//  - we know all exact multipart mime's
//  - we know that single part mimes are single part
//  - we don't know what mime a single part mime is (except that it's type is not multipart)
//
// changes:
//   1. (done) Resource::mime: Mime => SinglepartMime
//   2. (done) remove Builder/MimeLink "magic"
//   3. (done) add methodes:  GenericBuilder::multipart( Mime ), GenericBuilder::singlepart( Resource )
//   4. (done) remove SubBody and produce warnings at a different time
//       - we could just check all child headers when running MultipartBuilder::build
//       - even if it multipart in multipart it's fine to only check the layer below
//         as it will check it's layer below by itself
//   5. (done) split Elswhere into FileLoader+RunElswhere and E: Elsewhere => E: FileLoader+RunElswhere
//   6. (done) builder tree as seen below
//
//  Builder
//     .multipart( MultipartMime ) -> MultipartBuilder
//          .add_header( Header )
//          .add_body( |builder| builder.singlepart( ... )...build() )
//          .add_body( |builder| builder.multipart( Mime )...build() )
//          .build()
//     .singlepart( Resource ) -> SinglePartBuilder
//          .add_header( Header )
//          .build()
//
//
//


use std::ops::Deref;
use std::sync::Arc;
use std::borrow::Cow;
use std::path::Path;

use mime::Mime;
use futures::{ Future, IntoFuture };
use futures::future::{ self,  BoxFuture };

use error::*;
use types::TransferEncoding;
use headers::Header;
use codec::transfer_encoding::TransferEncodedFileBuffer;
use utils::FileBuffer;

use super::mime::MultipartMime;
use super::resource::Resource;
use super::{ MailPart, Mail, Headers, Body };
use super::utils::is_multipart_mime;


pub trait FileLoader {
    type FileFuture: Future<Item=Vec<u8>, Error=Error> + Send + 'static;
    /// load file specified by path, wile it returns
    /// a future it is not required to load the file
    /// in the background, as such you should not relay
    /// on it beeing non-blocking, it might just load
    /// the file in place and return futures::ok
    fn load_file( &self, path: &Path ) -> Self::FileFuture;
}

impl<F: FileLoader> FileLoader for Arc<F> {
    type FileFuture = F::FileFuture;
    fn load_file( &self, path: &Path ) -> Self::FileFuture {
        (*self).load_file( path )
    }
}

pub trait RunElsewhere {
    /// executes the futures `fut` "elswhere" e.g. in a cpu pool
    fn execute<F>( &self, fut: F) -> BoxFuture<Item=F::Item, Error=F::Error>
        where F: Future + Send + 'static,
              F::Item: Send+'static,
              F::Error: Send+'static;

    fn execute_fn<FN, I>( &self, fut: FN ) -> BoxFuture<Item=I::Item, Error=I::Error>
        where FN: FnOnce() -> I + Send + 'static,
              I: IntoFuture + 'static,
              I::Future: Send + 'static,
              I::Item: Send + 'static,
              I::Error: Send + 'static
    {
        self.execute( future::lazy( fut ) )
    }
}

impl<F: RunElsewhere> RunElsewhere for Arc<F> {
    fn execute<F>( &self, fut: F) -> BoxFuture<Item=F::Item, Error=F::Error>
        where F: Future + Send + 'static,
              F::Item: Send+'static,
              F::Error: Send+'static
    {
        (*self).execute( fut )
    }
}

trait BuilderContext: FileLoader+RunElsewhere+Clone {}
impl<T> BuilderContext for T where T: FileLoader+RunElsewhere+Clone {}


pub struct Builder<E: BuilderContext>(pub E);

struct BuilderShared<E: BuilderContext> {
    ctx: E,
    headers: Headers
}

pub struct SinglepartBuilder<E: BuilderContext> {
    inner: BuilderShared<E>,
    body: Resource
}

pub struct MultipartBuilder<E: BuilderContext> {
    inner: BuilderShared<E>,
    hidden_text: Option<String>,
    bodies: Vec<Mail>
}

impl<E: BuilderContext> BuilderShared<E> {

    fn new( ctx: E ) -> Self {
        BuilderShared {
            ctx,
            headers: Headers::new(),
        }
    }

    fn set_header( &mut self, header: Header, is_multipart: bool ) -> Result<Option<Header>> {
        //move checks for single/multipart from mail_composition here
        match &header {
            //FIXME check if forbidding setting ContentType/ContentTransferEncoding headers
            // is preferable, especially if is_multipart == false
            &ContentType( ref mime ) => {
                if is_multipart != is_multipart_mime( mime ) {
                    return Err( ErrorKind::ContentTypeAndBodyIncompatible.into() )
                }
            },
            _ => {}
        }

        let name = header.name().into();

        Ok( self.headers.insert( name, header ) )
    }

    fn set_headers<IT>( &mut self, iter: IT, is_multipart: bool ) -> Result<()>
        where IT: Iterator<Item=Header>
    {
        for header in iter {
            self.set_header( header, is_multipart )?
        }
        Ok( () )
    }

    fn build( self, body: MailPart ) -> Result<Mail> {
        Ok( Mail {
            headers: self.headers,
            body: body,
        } )
    }
}

impl<E: BuilderContext> Builder<E> {

    pub fn multipart( &self,  m: MultipartMime ) -> MultipartBuilder {
        MultipartBuilder {
            inner: BuilderShared::new( self.0.clone() ),
            hidden_text: None,
            bodies: Vec::new(),
        }
    }

    pub fn singlepart( &self, r: Resource ) -> SinglepartBuilder {
        SinglepartBuilder {
            inner: BuilderShared::new( self.0.clone() ),
            body: Resource,
        }
    }

}

impl<E: BuilderContext> SinglepartBuilder<E> {
    pub fn set_header( self, header: Header ) -> Result<Self> {
        self.inner.set_header( header, false )?;
        Ok( self )
    }

    pub fn set_headers<IT>( &mut self, iter: IT ) -> Result<Self>
        where IT: Iterator<Item=Header>
    {
        self.inner.set_headers( iter, false )?;
        Ok( self )

    }

    pub fn build( self ) -> Result<Mail> {
        use self::Resource::*;

        let body: Body = match self.body {
            FileBuffer( buffer ) => {
                self.inner.ctx.execute_fn(
                    move || TransferEncodedFileBuffer::encode_buffer( buffer, None )
                )
            },
            Future( future ) => {
                future.and_then( |buffer|
                    self.inner.ctx.execute_fn(
                        move || TransferEncodedFileBuffer::encode_buffer( buffer, None )
                    )
                ).into()
            },
            File { mime, path, alternate_name } => {
                self.inner.ctx.execute(
                    self.inner.e.load_file( path ).map( |data| {
                        //TODO add file meta, replacing name with alternate_name (if it is some)
                        let buffer = FileBuffer::new( mime, data );
                        TransferEncodedFileBuffer::encode_buffer( buffer, None )
                    })
                ).into()
            }
        };

        self.0.build( MailPart::SingleBody { body } )
    }
}

impl<E: BuilderContext> MultipartBuilder<E> {
    pub fn add_body<FN>( self, body_fn: FN ) -> Result<Self>
        where FN: FnOnce( &Builder<E> ) -> Result<Mail>
    {
        self.bodies.push( body_fn( &self.inner.ctx )? );
        self
    }

    pub fn set_headers<IT>( &mut self, iter: IT ) -> Result<Self>
        where IT: Iterator<Item=Header>
    {
        self.inner.set_headers( iter, true )?;
        Ok( self )

    }

    pub fn set_header( &mut self, header: Header ) -> Result<Self> {
        self.inner.set_header( header, true )?;
        Ok( self )
    }

    pub fn build( self ) -> Result<Mail> {
        if self.bodies.len() == 0 {
            Err( ErrorKind::NeedAtLastOneBodyInMultipartMail.into() )
        } else {
            self.inner.build( MailPart::MultipleBodies {
                bodies: self.bodies,
                hidden_text: self.hidden_text.unwrap_or( String::new() ),
            } )
        }
    }
}