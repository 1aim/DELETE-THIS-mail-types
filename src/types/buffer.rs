use std::ops::Deref;

use mime::{ Mime, TEXT, CHARSET };

use super::FileMeta;

// WHEN_FEATURE(more_charsets)
// for now this is just a vector,
// but when <encodings> is used to support
// non-utf8/non-ascii encodings this will
// have more fields, like e.g. `encoding: EncodingSpec`
pub struct FileBuffer {
    content_type: Mime,
    data: Vec<u8>,
    file_meta: FileMeta
}


impl FileBuffer {

    pub fn new( content_type: Mime, data: Vec<u8> ) -> FileBuffer {
        FileBuffer::new_with_file_meta( content_type, data, Default::default() )
    }

    pub fn new_with_file_meta( content_type: Mime, data: Vec<u8>, file_meta: FileMeta ) -> FileBuffer {
        FileBuffer { content_type, data, file_meta }
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

impl Deref for FileBuffer {
    type Target = [u8];
    fn deref( &self ) -> &[u8] {
        *self.data
    }
}

impl Into< Vec<u8> > for FileBuffer {
    fn into(self) -> Vec<u8> {
        self.data
    }
}

