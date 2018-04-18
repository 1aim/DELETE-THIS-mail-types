use std::ops::Deref;

use media_type::{TEXT, CHARSET};

use common::utils::FileMeta;
use common::bind::{quoted_printable, base64};
use common::error::{EncodingError, UNKNOWN};

use headers::components::{TransferEncoding, MediaType};



// WHEN_FEATURE(more_charsets)
// for now this is just a vector,
// but when <encodings> is used to support
// non-utf8/non-ascii encodings this will
// have more fields, like e.g. `encoding: EncodingSpec`
#[derive(Debug, Clone)]
pub struct FileBuffer {
    content_type: MediaType,
    data: Vec<u8>,
    file_meta: FileMeta
}


impl FileBuffer {

    pub fn new(content_type: MediaType, data: Vec<u8>) -> FileBuffer {
        FileBuffer::with_file_meta(content_type, data, Default::default() )
    }

    pub fn with_file_meta(content_type: MediaType, data: Vec<u8>, file_meta: FileMeta )
        -> FileBuffer
    {
        FileBuffer { content_type, data, file_meta }
    }

    pub fn with_data<FN>(mut self, modif: FN) -> Self
        where FN: FnOnce( Vec<u8> ) -> Vec<u8>
    {
        self.data = modif(self.data);
        self
    }

    pub fn content_type(&self) -> &MediaType {
        &self.content_type
    }

    pub fn file_meta(&self) -> &FileMeta {
        &self.file_meta
    }

    pub fn file_meta_mut(&mut self) -> &mut FileMeta {
        &mut self.file_meta
    }

    pub fn has_ascii_charset(&self) -> bool {
        let ct = self.content_type();
        ct.type_() == TEXT &&
            ct.get_param(CHARSET)
                .map(|charset| charset == "us-ascii")
                .unwrap_or(true)
    }

    pub fn contains_text(&self) -> bool {
        let type_ = self.content_type().type_();
        type_ == TEXT
    }

}

impl Deref for FileBuffer {
    type Target = [u8];
    fn deref( &self ) -> &[u8] {
        &*self.data
    }
}

impl Into< Vec<u8> > for FileBuffer {
    fn into(self) -> Vec<u8> {
        self.data
    }
}




pub fn find_encoding(buffer: &FileBuffer) -> TransferEncoding {
    if buffer.has_ascii_charset() {
        //TODO support lossy 7Bit encoding dropping '\0' and orphan '\n', '\r'
        TransferEncoding::_7Bit
    } else if buffer.contains_text() {
        TransferEncoding::QuotedPrintable
    } else {
        TransferEncoding::Base64
    }
}


#[derive(Debug, Clone)]
pub struct TransferEncodedFileBuffer {
    inner: FileBuffer,
    encoding: TransferEncoding
}

impl TransferEncodedFileBuffer {
    pub fn buffer_is_encoded( buf: FileBuffer, with_encoding: TransferEncoding ) -> Self {
        TransferEncodedFileBuffer {
            inner: buf,
            encoding: with_encoding
        }
    }

    pub fn transfer_encoding( &self ) -> &TransferEncoding {
        &self.encoding
    }

    /// transforms a unencoded FileBuffer into a TransferEncodedFileBuffer
    ///
    /// if a preferred_encoding is given it is used,
    /// else if the buffer has a ascii charset 7Bit encoding is used
    /// else if the buffer contains text quoted-printable is used
    /// else base64 encoding is used
    pub fn encode_buffer(
        buffer: FileBuffer,
        //Note: TransferEncoding is Copy
        preferred_encoding: Option<TransferEncoding>
    ) -> Result<TransferEncodedFileBuffer, EncodingError>
    {
        use self::TransferEncoding::*;

        let encoding = preferred_encoding
            .unwrap_or_else(|| find_encoding(&buffer));


        match encoding {
            _7Bit => encode_7bit( buffer ),
            _8Bit => encode_8bit( buffer ),
            Binary => encode_binary( buffer ),
            QuotedPrintable => encode_quoted_printable( buffer ),
            Base64 => encode_base64( buffer ),
        }
    }

    pub fn as_slice(&self) -> &[u8] {
        self
    }
}


impl Deref for TransferEncodedFileBuffer {
    type Target = FileBuffer;
    fn deref( &self ) -> &FileBuffer {
        &self.inner
    }
}



fn encode_7bit(buffer: FileBuffer) -> Result<TransferEncodedFileBuffer, EncodingError> {
    {
        let data: &[u8] = &*buffer;

        let mut last = b'\0';
        for byte in data.iter().cloned() {
            if byte >= 128 {
                ec_bail!(kind: InvalidTextEncoding {
                    expected_encoding: "7-bit"
                    got_encoding: UNKNOWN
                })
            }
            if byte == 0 || ((last == b'\r') != (byte == b'\n')) {
                ec_bail!(kind: Malformed)
            }
            last = byte;
        }
    }
    Ok(TransferEncodedFileBuffer::buffer_is_encoded( buffer, TransferEncoding::_7Bit))
}

fn encode_8bit(buffer: FileBuffer) -> Result<TransferEncodedFileBuffer, EncodingError> {
    {
        let data: &[u8] = &*buffer;

        let mut last = b'\0';
        for byte in data.iter().cloned() {
            if byte == 0 || (( last==b'\r' ) != (byte == b'\n')) {
                ec_bail!(kind: Malformed)
            }
            last = byte;
        }
    }
    Ok(TransferEncodedFileBuffer::buffer_is_encoded(buffer, TransferEncoding::_8Bit))
}

/// to quote RFC 2045:
/// """[at time of writing] there are no standardized Internet mail transports
///    for which it is legitimate to include
///    unencoded binary data in mail bodies. [...]"""
///
/// nevertheless there is at last one SMTP extension which allows this
/// (chunked),but this library does not support it for now
fn encode_binary(buffer: FileBuffer) -> Result<TransferEncodedFileBuffer, EncodingError> {
    Ok( TransferEncodedFileBuffer::buffer_is_encoded(buffer, TransferEncoding::Binary))
}

fn encode_quoted_printable(buffer: FileBuffer)
    -> Result<TransferEncodedFileBuffer, EncodingError>
{
    Ok(TransferEncodedFileBuffer::buffer_is_encoded(
        buffer.with_data(|data| quoted_printable::normal_encode(data).into()),
        TransferEncoding::QuotedPrintable
    ))
}

fn encode_base64( buffer: FileBuffer )
    -> Result<TransferEncodedFileBuffer, EncodingError>
{
    Ok(TransferEncodedFileBuffer::buffer_is_encoded(
        buffer.with_data(|data| base64::normal_encode(data).into_bytes()),
        TransferEncoding::Base64
    ))
}

