use chrono;
use ascii::AsciiStr;

use error::*;
use codec::{ MailEncoder, MailEncodable };

pub use utils::DateTime;

impl MailEncodable for DateTime {
    fn encode( &self, encoder: &mut MailEncoder ) -> Result<()> {
        let as_str = self.0.to_rfc2822();
        let ascii = unsafe { AsciiStr::from_ascii_unchecked( &*as_str ) };
        encoder.write_str( ascii );
        Ok( () )
    }
}