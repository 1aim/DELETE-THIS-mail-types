
use ascii::{  AsciiChar, AsciiStr };

use error::*;
use super::address_list_header;
use types::{ AddressList };
use codec::{ MailDecodable, MailEncodable, MailEncoder };

pub struct Bcc( AddressList );

impl Header for Bcc {
    fn name() -> &'static AsciiStr {
        ascii_str! { B c c }
    }
}

impl MailEncodable for Bcc {
    fn encode( &self, encoder: &mut MailEncoder ) -> Result<()> {
        address_list_header::encode( Self::name(), &self.0, encoder )
    }
}

impl MailDecodable for Bcc {

    fn decode( _encoded: &str ) -> Result<Bcc> {
        unimplemented!();
    }
}
