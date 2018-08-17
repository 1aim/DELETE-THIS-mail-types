//! Provides the `FileBuffer` type.
//!
//! The file buffer is a buffer (i.e. some bytes)
//! combined with an `FileMeta`` instance and a `MediaType`.
//! The file buffer is used by the `Resource`
//! type to represent the content of bodies of mails.
//! As such it's mainly used for embedded images and
//! attachments but also happens to be used for "one
//! the file" generated text bodies. I.e. it represent
//! something which could be a file, but might not
//! be backed by an "on disk" file representation.
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
/// A byte buffer containing the file content and associated `FileMeta` and `MediaType`.
///
/// While it's called file buffer, it doesn't need to be backed by a file on some file
/// system, it just has the file metadata and an media type.
#[derive(Debug, Clone)]
pub struct FileBuffer {
    content_type: MediaType,
    data: Vec<u8>,
    file_meta: FileMeta
}


impl FileBuffer {

    /// Create a new buffer from a media type and a byte buffer.
    ///
    /// The created file buffer will have default values for all
    /// file metadata (i.e. `None`).
    pub fn new(content_type: MediaType, data: Vec<u8>) -> FileBuffer {
        FileBuffer::with_file_meta(content_type, data, Default::default())
    }

    /// Creates a new buffer from data, a media type and a `FileMeta` instance.
    pub fn with_file_meta(content_type: MediaType, data: Vec<u8>, file_meta: FileMeta )
        -> FileBuffer
    {
        FileBuffer { content_type, data, file_meta }
    }

    //TODO[NOW]: replace with data_mut()?
    /// Allows the modification of the contained data.
    pub fn with_data<FN>(mut self, modif: FN) -> Self
        where FN: FnOnce(Vec<u8>) -> Vec<u8>
    {
        self.data = modif(self.data);
        self
    }

    /// Returns the content type associated with the buffer.
    pub fn content_type(&self) -> &MediaType {
        &self.content_type
    }

    /// Returns a reference to the file meta data.
    pub fn file_meta(&self) -> &FileMeta {
        &self.file_meta
    }

    /// Returns a mutable reference to the file meta data.
    pub fn file_meta_mut(&mut self) -> &mut FileMeta {
        &mut self.file_meta
    }

    /// Returns true if the content type is a `text` type and he charset is `us-ascii`.
    pub fn has_ascii_charset(&self) -> bool {
        let ct = self.content_type();
        ct.type_() == TEXT &&
            ct.get_param(CHARSET)
                .map(|charset| charset == "us-ascii")
                .unwrap_or(true)
    }

    /// Returns true if the content type is a `text` type.
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



/// Tries to find a good content transfer encoding for the buffer.
///
/// For most data this will return `Base64`.
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

/// A version of an file buffer where the content had been transfer encoded.
#[derive(Debug, Clone)]
pub struct TransferEncodedFileBuffer {
    inner: FileBuffer,
    encoding: TransferEncoding
}

impl TransferEncodedFileBuffer {

    /// Creates an buffer assuming it's data to be encoded with given content transfer encoding.
    pub fn buffer_is_encoded(buf: FileBuffer, with_encoding: TransferEncoding) -> Self {
        TransferEncodedFileBuffer {
            inner: buf,
            encoding: with_encoding
        }
    }

    /// Returns the content transfer encoding used when encoding this buffer.
    pub fn transfer_encoding( &self ) -> &TransferEncoding {
        &self.encoding
    }

    /// Transforms a unencoded FileBuffer into a TransferEncodedFileBuffer.
    ///
    /// - If a preferred_encoding is given it is used.
    /// - Else if the buffer has a ascii charset 7Bit encoding is used.
    /// - Else if the buffer contains text quoted-printable is used.
    /// - Else base64 encoding is used.
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

    /// Returns the content of the buffer as byte slice.
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
                    expected_encoding: "7-bit",
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

