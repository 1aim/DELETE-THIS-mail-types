use std::ops::Deref;

use mime::Name;
use futures::BoxFuture;

use types::TransferEncoding;

trait PushIfSome<T> {
    fn push_if_some( &mut self, val: Option<T> );
}

impl<T> PushIfSome<T> for Vec<T> {
    #[inline]
    fn push_if_some( &mut self, val: Option<T> ) {
        if let Some( val ) = val {
            self.push( val );
        }
    }
}


type BufferFuture = BoxFuture<Item=Buffer, Error=Error>;


pub enum MimeBitDomain {
    /// 7bit Ascii (US-ASCII), no \0, no orphan `\n`, `\r`, line length limiations
    _7Bit,
    /// 8bit, no \0, no orphan `\n`, `\r`, line length limiations
    ///
    /// Note that for now _8Bit isn't really used at all, everything
    /// with is not 7bit is currently labeled as Binary
    _8Bit,
    /// binary, not constraints
    ///
    /// (through doing dot-statching on smtp level, and
    ///   boundary checks for mime multipart boundaries
    ///   is still nessesary)
    Binary
}


// WHEN_FEATURE(more_charsets)
// for now this is just a vector,
// but when <encodings> is used to support
// non-utf8/non-ascii encodings this will
// have more fields, like e.g. `encoding: EncodingSpec`
pub struct Buffer {
    inner: Vec<u8>,
    content_type: Mime,
    //file_meat_info: FMI // like name, cration_data, etc.
    transfer_encoding: Option<TransferEncoding>
}


impl Buffer {

    fn bit_domain( &self ) -> MimeBitDomain {
        let is_7bit = self.charset()
            .map( |charset| charset == "us-ascii" )
            .unwrap_or( false );

        if is_7bit {
            MimeBitDomain::_7Bit
        } else {
            MimeBitDomain::Binary
        }
    }

    fn charset<'a>( &'a self ) -> Option<mime::Name<'a>> {
        self.content_type.get_param( mime::CHARSET )
    }

    fn is_test( &self ) -> bool {
        self.content_type.type_() == mime::TEXT
    }

    fn set_content_transfer_encoding( &mut self, encoding: TransferEncoding ) {
        self.transfer_encoding = encoding;
    }

    fn content_transfer_encoding( &self ) -> Option<&TransferEncoding> {
        self.transfer_encoding.as_ref()
    }

}

impl Deref for Buffer {
    type Target = [u8];
    fn deref( &self ) -> &[u8] {
        *self.inner
    }
}

impl Into< Vec<u8> > for Buffer {
    fn into(self) -> Vec<u8> {
        self.inner
    }
}

impl From< Vec<u8> > for Buffer {
    fn from( data: Vec<u8> ) -> Self {
        Buffer { inner: data }
    }
}