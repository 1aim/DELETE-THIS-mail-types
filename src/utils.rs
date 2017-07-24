use std::ops::Deref;

use chrono;
use mime::{ Mime, Name, CHARSET, TEXT };
use ascii::AsciiString;
use futures::BoxFuture;

use types::TransferEncoding;

//trait PushIfSome<T> {
//    fn push_if_some( &mut self, val: Option<T> );
//}
//
//impl<T> PushIfSome<T> for Vec<T> {
//    #[inline]
//    fn push_if_some( &mut self, val: Option<T> ) {
//        if let Some( val ) = val {
//            self.push( val );
//        }
//    }
//}


pub struct DateTime( chrono::DateTime<chrono::Utc> );

impl DateTime {
    fn new<TZ: chrono::TimeZone>( date_time: chrono::DateTime<TZ>) -> DateTime {
        DateTime( date_time.with_timezone( &chrono::Utc ) )
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Hash, Default)]
pub struct  FileMeta {
    // in rust std this is OsString, but we can not have it
    // os specific in any way, as it is send over internet,
    // originally this was Ascii, but has been extended
    // to support encoding
    // FEATURE_TODO(utf8_file_names): AsciiString => String
    pub file_name: Option<AsciiString>,
    pub creation_date: Option<DateTime>,
    pub modification_date: Option<DateTime>,
    pub read_date: Option<DateTime>,
    pub size: Option<usize>
}

// WHEN_FEATURE(more_charsets)
// for now this is just a vector,
// but when <encodings> is used to support
// non-utf8/non-ascii encodings this will
// have more fields, like e.g. `encoding: EncodingSpec`
pub struct Buffer {
    content_type: Mime,
    data: Vec<u8>,
    file_meta: FileMeta
}


impl Buffer {

    pub fn new( content_type: Mime, data: Vec<u8> ) -> Buffer {
        Buffer::new_with_file_meta( content_type, data, Default::default() )
    }

    pub fn new_with_file_meta( content_type: Mime, data: Vec<u8>, file_meta: FileMeta ) -> Buffer {
        Buffer { content_type, data, file_meta }
    }

    pub fn with_data<FN>( self, modif: FN ) -> Self
        where FN: FnOnce( Vec<u8> ) -> Vec<u8>
    {
        self.data = modif( self.data );
        self
    }

    pub fn content_type( &self ) -> &Mime {
        &self.content_type
    }

    pub fn file_meta( &self ) -> &FileMeta {
        &self.file_meta
    }

    pub fn file_meta_mut( &self ) -> &mut FileMeta {
        &mut self.file_meta
    }

    pub fn has_ascii_charset( &self ) -> bool {
        self.content_type()
            .get_param( CHARSET )
            .map( |charset| charset == "us-ascii" )
            .unwrap_or( false )
    }

    pub fn contains_text( &self ) -> bool {
        let type_ = self.content_type().type_();
        type_ == TEXT
    }

}

impl Deref for Buffer {
    type Target = [u8];
    fn deref( &self ) -> &[u8] {
        *self.data
    }
}

impl Into< Vec<u8> > for Buffer {
    fn into(self) -> Vec<u8> {
        self.data
    }
}

