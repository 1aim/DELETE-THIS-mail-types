//use FnvHashMap
use base64;
use quoted_printable;
use std::collections::{ HashMap as Map };
use std::sync::Mutex;

use futures::stream::Stream;

use error::*;
use utils::{ Buffer, BufferFuture };
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

pub type ByteStream = BoxStream<Item=u8, Error=Error>;

//WHEN_FEATURE(check_multipart_boundaries)
// change it to fn(Buffer, Boundary) -> Result<Buffer>
pub type TransferEncoder = fn(Buffer) -> Result<Buffer>;

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





fn encode_7bit( buf: Buffer ) -> Buffer {
    let data: &[u8] = &*buf;

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

}

fn encode_8bit( stream: ByteStream ) -> ByteStream {
    let data: &[u8] = &*buf;

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
}

/// to quote RFC 2045:
/// """[at time of writing] there are no standardized Internet mail transports
///    for which it is legitimate to include
///    unencoded binary data in mail bodies. [...]"""
///
/// nevertheless there is at last one SMTP extension which allows this
/// (chunked),but this library does not support it for now
fn encode_binary( buffer: Buffer ) -> Buffer {
    buffer
}

fn encode_quoted_printable( buffer: Buffer ) -> Buffer {
    quoted_printable::encode( &*buffer ).into()
}

fn encode_base64( buffer: Buffer ) -> Buffer {
    base64::encode_config( &*buffer, base64::MIME )
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