use ascii::{ AsciiChar, AsciiString };

use error::*;
use codec::{ MailEncoder, MailEncodable };
use codec::utf8_to_ascii::puny_code_domain;
use char_validators::{ is_atext, is_qtext, is_vchar, is_ws, MailType };


use super::utils::item::{ SimpleItem, Input };

/// an email of the form `local-part@domain`
/// corresponds to RFC5322 addr-spec, so `<`, `>` padding is _not_
/// part of this Email type (but of the Mailbox type instead)
#[derive(Debug,  Clone, Hash, PartialEq, Eq)]
pub struct Email {
    pub local_part: LocalPart,
    pub domain: Domain
}


#[derive(Debug,  Clone, Hash, PartialEq, Eq)]
pub struct LocalPart( SimpleItem );


#[derive(Debug,  Clone, Hash, PartialEq, Eq)]
pub struct Domain( SimpleItem );



impl MailEncodable for Email {

    fn encode<E>( &self, encoder: &mut E ) -> Result<()>
        where E: MailEncoder
    {
        local_part.encode( encoder )?;
        encoder.write_char( AsciiChar::At );
        domain.encode( encoder )?
    }

}

impl LocalPart {

    pub fn from_input( input: Input ) -> Self {
        let mut requires_quoting = false;
        let mut mail_type = MailType::Ascii;
        for char in input.chars() {
            if !is_atext( char, mail_type ) {
                if char.len_utf8() > 0 {
                    mail_type = MailType::Internationalized;
                    if is_atext( char, mail_type ) {
                        continue;
                    }
                }
                requires_quoting = true;
            }
        }
        let new_input = if requires_quoting {
            Input::Owned( quote( &*input ) )
        } else {
            input
        };

        LocalPart( match mail_type {
            MailType::Internationalized => SimpleItem::Utf8( input.into_utf8_item() ),
            MailType::Ascii => {
                //OPTIMIZE: it should be guaranteed to be ascii
                //SimpleItem::Ascii( unsafe { input.into_ascii_item_unchecked() } )
                SimpleItem::Ascii( input.into_ascii_item().unwrap() )
            }
        } )
    }
}

impl MailEncodable for LocalPart {
    fn encode<E>( &self, encoder: &mut E ) -> Result<()>
        where E: MailEncoder
    {
        use super::utils::item::Item::*;
        match *self.0 {
            Ascii( ascii ) => {
                encoder.write_str( ascii );
            },
            Utf8( utf8 ) => {
                encoder.try_write_utf8( utf8 )?;
            }
        }
    }
}

impl Domain {
    pub fn from_input( inp: Input ) -> Self {
        let string = match inp {
            Input::Owned( string ) => string,
            Input::Shared( ref_to_string ) => String::from( &*ref_to_string ),
        };

        Domain( match string.into_ascii_string() {
            Ok( ascii ) => SimpleItem::Ascii( ascii ),
            Err( ascii_err ) => SimpleItem::Utf8( ascii_err.owner )
        } )
    }
}

impl MailEncodable for Domain {
    fn encode<E>( &self, encoder: &mut E ) -> Result<()>
        where E: MailEncoder
    {
        match *domain.0 {
            SimpleItem::Ascii( ref ascii ) => {
                encoder.write_str( ascii )
            },
            SimpleItem::Utf8( ref utf8 ) => {
                if encoder.try_write_utf8( utf8 ).is_err() {
                    puny_code_domain( utf8, encoder );
                }
            }
        }
    }
}


fn quote( input: &str ) -> Result<String> {
    let mut out = String::new();
    for char in input.chars() {
        if is_qtext( char, MailType::Internationalized ) {
           out.push( char )
        } else {
            //NOTE: while quoting ws is possible it is not nessesary as
            // a quoted string can contain FWS, and only CRLF in a quoted
            // string are semantically invisible (meaning the WSP after
            // CRLF _is_ semantically visible)
            if is_vchar( char, MailType::Internationalized) || is_ws( char ) {
                out.push( '\\' );
                out.push( char );
            } else {
                // char: 0-31
                bail!( "can not quote char: {:?}", char );
            }
        }
    }
}
