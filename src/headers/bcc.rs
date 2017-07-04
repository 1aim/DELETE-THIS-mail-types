
use ascii::{  AsciiChar, AsciiStr };

use error::*;
use super::address_list_header;
use types::{ AddressList };
use codec::{ SmtpDataDecodable, SmtpDataEncodable, SmtpDataEncoder };

pub struct Bcc( AddressList );

impl Header for Bcc {
    fn name() -> &'static AsciiStr {
        ascii_str! { B c c }
    }
}

impl SmtpDataEncodable for Bcc {
    fn encode( &self, encoder: &mut SmtpDataEncoder ) -> Result<()> {
        address_list_header::encode( Self::name(), &self.0, encoder )
    }
}

impl SmtpDataDecodable for Bcc {

    fn decode( _encoded: &str ) -> Result<Bcc> {
        unimplemented!();
    }
}
