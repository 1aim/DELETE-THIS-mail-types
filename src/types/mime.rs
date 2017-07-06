use mime;
use ascii::AsciiStr;

use error::*;
use codec::{ SmtpDataEncoder, SmtpDataEncodable };

pub use mime::Mime;


impl SmtpDataEncodable for mime::Mime {

    fn encode( &self, encoder: &mut SmtpDataEncoder ) -> Result<()> {
        let res = self.to_string();
        //FIXME expose a unsafe write_str_as_ascii_unchecked ?
        encoder.write_str( unsafe { AsciiStr::from_ascii_unchecked( &*res ) } );
        Ok( () )
    }
}
