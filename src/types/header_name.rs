use ascii::{ AsciiString, AsciiChar };

use codec::{ SmtpDataEncodable, SmtpDataEncoder };
use error::*;
// we need this for the `Other` and `ContentTypeExtension`
// cases when they are used for generating mails
// this is not meant to be used in a getter, consider using
// e.g. `name() -> &'static AsciiStr` instead


pub struct HeaderName( AsciiString );

impl HeaderName {
    fn new( name: String ) -> Result<HeaderName> {
        for char in name.iter() {
            match char {
                'a'...'z' |
                'A'...'Z' |
                '0'...'9' |
                '-' => {},
                _ => return Err( ErrorKind::InvalidHeaderName( name ).into() )
            }
        }
        HeaderName( unsafe { AsciiString::from_ascii_unchecked( name ) } )
    }
}


impl SmtpDataEncodable for HeaderName {
    fn encode( &self, encoder: &mut SmtpDataEncoder ) -> Result<()> {
        encoder.write_str( &*self.0 );
        encoder.write_char( AsciiChar::Colon );
        Ok( () )
    }
}
