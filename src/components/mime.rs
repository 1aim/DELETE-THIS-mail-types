use mime;
use ascii::AsciiStr;

use error::*;
use codec::{MailEncoderImpl, MailEncodable };

pub use mime::Mime;


impl MailEncodable for mime::Mime {

    fn encode<E>( &self, encoder:  &mut E ) -> Result<()> where E: MailEncoder {
        let res = self.to_string();
        //FIXME expose a unsafe write_str_as_ascii_unchecked ?
        encoder.write_str( unsafe { AsciiStr::from_ascii_unchecked( &*res ) } );
        Ok( () )
    }
}
