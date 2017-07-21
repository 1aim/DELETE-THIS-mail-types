use std::ops::Deref;

use futures::BoxFuture;

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


// WHEN_FEATURE(more_charsets)
// for now this is just a vector,
// but when <encodings> is used to support
// non-utf8/non-ascii encodings this will
// have more fields, like e.g. `encoding: EncodingSpec`
pub struct Buffer {
    inner: Vec<u8>
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