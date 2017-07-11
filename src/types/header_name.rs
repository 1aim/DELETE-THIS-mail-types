use std::ops::Deref;

use ascii::{ AsciiString, AsciiStr, AsciiChar };

use codec::{ MailEncodable, MailEncoder };
use error::*;
// we need this for the `Other` and `ContentTypeExtension`
// cases when they are used for generating mails
// this is not meant to be used in a getter, consider using
// e.g. `name() -> &'static AsciiStr` instead


pub struct HeaderName( AsciiString );

impl HeaderName {
    pub fn new( name: String ) -> Result<HeaderName> {
        let mut ok = true;
        for char in name.chars() {
            let ok = match char {
                'a'...'z' |
                'A'...'Z' |
                '0'...'9' |
                '-' => {},
                _ => { ok = false; break; }
            };
        }
        if ok {
            Ok( HeaderName( unsafe { AsciiString::from_ascii_unchecked( name ) } ) )
        } else {
            Err(ErrorKind::InvalidHeaderName(name).into())
        }
    }
}


impl MailEncodable for HeaderName {
    fn encode( &self, encoder: &mut MailEncoder ) -> Result<()> {
        encoder.write_str( &*self.0 );
        encoder.write_char( AsciiChar::Colon );
        Ok( () )
    }
}

impl Deref for HeaderName {
    type Target = AsciiStr;
    fn deref( &self ) -> &AsciiStr {
        &*self.0
    }
}