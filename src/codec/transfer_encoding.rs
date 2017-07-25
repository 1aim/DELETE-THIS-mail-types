use std::ops::Deref;

use mime::{ Mime, CHARSET, TEXT };
use base64;
use quoted_printable;
//use FnvHashMap
use std::collections::{ HashMap as Map };
use std::sync::Mutex;

use futures::{ future, Future };

use error::*;
use utils::{ FileBuffer, FileBufferFuture };
use types::TransferEncoding;


lazy_static! {
    static ref TRANSFER_ENCODING_EXTENSIONS:
        Mutex<Option<HashMap<TransferEncoding, TransferEncoder>>> = {
            use types::TransferEncoding::*;
            let mut encoders = HashMap::new();
            encoders.insert( _7Bit, encode_7bit );
            encoders.insert( _8Bit, encode_8bit );
            encoders.insert( Binary, encode_binary );
            encoders.insert( QuotedPrintable, encode_quoted_printable );
            encoders.insert( Base64, encode_base_64 )
            Mutex::new( Some( encoders ) )
        };

    pub static ref TRANSFER_ENCODINGS: EncoderStore = EncoderStore::create();
}


//WHEN_FEATURE(check_multipart_boundaries)
// change it to fn(FileBuffer, Boundary) -> Result<FileBuffer>
pub type TransferEncoder = fn(FileBuffer) -> FileBufferFuture;

pub struct EncoderStore {
    encoders: Map<TransferEncoding, EncodeStreamFn>,
}

impl EncoderStore {

    fn create() -> EncoderStore {
        let mut registry = TRANSFER_ENCODING_EXTENSIONS.lock().unwrap();
        let encoders = registry.take();
        EncoderStore { encoders }
    }

    fn register_extension( encoding: TransferEncoding, tencode: TransferEncoder ) -> Result<()> {
        let mut registry = TRANSFER_ENCODING_EXTENSIONS.lock().unwrap();
        if let Some( registry ) = registry.as_ref() {
            registry.insert( TransferEncoding, tencode );
            Ok( () )
        } else {
            Err( ErrorKind::RegisterExtensionsToLate( encoding.name().as_str().into() ).into() )
        }
    }

    fn lookup( &self, encoding: &TransferEncoding ) -> Result<TransferEncoder> {
        if let Some( tencoder ) = self.encoders.get( encoding ) {
            Ok( tencoder.clone() )
        } else {
            Err( ErrorKind::UnknownTransferEncoding( encoding.name().as_str().into() ))
        }
    }
}


pub struct TransferEncodedFileBuffer {
    inner: FileBuffer,
    encoding: TransferEncoding
}

impl TransferEncodedFileBuffer {
    fn buffer_is_encoded( buf: FileBuffer, with_encoding: TransferEncoding ) -> Self {
        TransferEncodedFileBuffer {
            inner: FileBuffer,
            encoding: TransferEncoding
        }
    }

    fn transfer_encoding( &self ) -> &TransferEncoding {
        &self.encoding
    }

    /// transforms a unencoded FileBuffer into a TransferEncodedFileBuffer
    ///
    /// if a preferred_encoder is given it is used,
    /// else if the buffer has a ascii charset 7Bit encoding is used
    /// else if the buffer contains text quoted-printable is used
    /// else base64 encoding is used
    fn encode_buffer(
        buffer: FileBuffer,
        preferred_encoder: Option<TransferEncoder>
    ) -> Result<TransferEncodedFileBuffer>
    {
        let func = if let Some( func ) = preferred_encoder {
            func
        } else {
            let encoding =
                if buffer.has_ascii_charset() {
                    //TODO support lossy 7Bit encoding dropping '\0' and orphan '\n', '\r'
                    TranserEncoding::_7Bit
                } else if buffer.contains_text() {
                    TransferEncoding::QuotedPrintable
                } else {
                    TransferEncoding::Base64
                };
            // This should never fail as _7Bit, QuotedPrintable and Base64 are always implemented
            TRANSFER_ENCODINGS.lookup( encoding )?
        };

        func( buffer )
    }

}



impl Deref for TransferEncodedFileBuffer {
    type Target = FileBuffer;
    fn deref( &self ) -> &FileBuffer {
        &self.data
    }
}



fn encode_7bit( mut buffer: FileBuffer ) -> Result<TransferEncodedFileBuffer> {
    let data: &[u8] = &*buffer;

    let mut last = b'\0';
    for byte in data {
        if byte >= 128 || byte == 0 {
            return Err( ErrorKind::Invalide7BitValue( byte ).into() )
        }
        if ( last==b'\r' ) != (byte == b'\n') {
            return Err( ErrorKind::Invalide7BitSeq( byte ).into() )
        }
        last = byte;
    }

    Ok( TransferEncodedFileBuffer::buffer_is_encoded( buffer, TransferEncoding::_7Bit ) )
}

fn encode_8bit( mut buffer: FileBuffer ) -> Result<TransferEncodedFileBuffer> {
    let data: &[u8] = &*buffer;

    let mut last = b'\0';
    for byte in data {
        if  byte == 0 {
            return Err( ErrorKind::Invalide8BitValue( byte ).into() )
        }
        if ( last==b'\r' ) != (byte == b'\n') {
            return Err( ErrorKind::Invalide8BitSeq( byte ).into() )
        }
        last = byte;
    }

    Ok( TransferEncodedFileBuffer::buffer_is_encoded( buffer, TransferEncoding::_8Bit ) )
}

/// to quote RFC 2045:
/// """[at time of writing] there are no standardized Internet mail transports
///    for which it is legitimate to include
///    unencoded binary data in mail bodies. [...]"""
///
/// nevertheless there is at last one SMTP extension which allows this
/// (chunked),but this library does not support it for now
fn encode_binary( mut buffer: FileBuffer ) -> Result<TransferEncodedFileBuffer> {
    Ok( TransferEncodedFileBuffer::buffer_is_encoded( buffer, TransferEncoding::Binary ) )
}

fn encode_quoted_printable( buffer: FileBuffer ) -> Result<TransferEncodedFileBuffer> {
    Ok( TransferEncodedFileBuffer::buffer_is_encoded(
        buffer.with_data( |data| quoted_printable::encode( &*data ) ),
        TransferEncoding::QuotedPrintable
    ) )
}

fn encode_base64( buffer: FileBuffer ) -> Result<TransferEncodedFileBuffer> {
    Ok( TransferEncodedFileBuffer::buffer_is_encoded(
        buffer.with_data( |data| base64::encode_config( &*data, base64::MIME ).into_bytes() ),
        TransferEncoding::Base64
    ) )
}


#[cfg(test)]
mod test {
    use super::*;

    fn assure_send<A: Send>() {}
    fn assure_clone<A: Clone>() {}

    #[test]
    fn compt_check_send_clone() {
        assure_clone::<TransferEncoder>();
        assure_send::<TransferEncoder>();
    }

}