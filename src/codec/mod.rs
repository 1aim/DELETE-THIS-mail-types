use std::fmt;
use std::result::{ Result as StdResult };
use std::ascii::AsciiExt;

use char_validators::{ is_atext, MailType };
use char_validators::encoded_word::EncodedWordContext;

use ascii::{ AsciiString, AsciiStr, AsciiChar };

use error::*;
use utils::insert_bytes;

pub mod transfer_encoding;
pub mod utf8_to_ascii;

use self::utf8_to_ascii::q_encode_for_encoded_word;

pub trait MailEncoder {
    fn mail_type( &self ) -> MailType;

    fn write_new_line( &mut self );
    fn write_fws( &mut self );
    fn note_optional_fws(&mut self );

    fn write_char( &mut self, char: AsciiChar );
    fn write_str( &mut self, str: &AsciiStr );

    fn try_write_atext( &mut self, str: &str ) -> Result<()>;
    fn write_encoded_word( &mut self, data: &str, ctx: EncodedWordContext );


    fn break_line_on_last_cfws( &mut self );
    fn current_line_byte_length(&self ) -> usize;
}

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct MailEncoderImpl {
    inner: Vec<u8>,
    current_line_byte_length: usize,
    last_cfws_pos: Option<usize>,
    mail_type: MailType
}


impl MailEncoderImpl {

    pub fn new(mail_type: MailType) -> MailEncoderImpl {

        MailEncoderImpl {
            mail_type: mail_type,
            inner: Vec::new(),
            current_line_byte_length: 0,
            last_cfws_pos: None
        }
    }

    fn write_byte_unchecked(&mut self, byte: u8 ) {
        //FIXME: potentially keep track of "line ending state" to prevent rogue '\r' '\n'
        //THIS IS THE ONLY FUNCTION WHICH SHOULD WRITE TO self.inner!!
        self.inner.push( byte );
        self.current_line_byte_length += 1;
        if byte == b'\n' && *self.inner.last().unwrap() == b'\r' {
            self.current_line_byte_length = 0;
            self.last_cfws_pos = None;
        }
    }

    fn write_data_unchecked(&mut self, data: &[u8] ) {
        for byte in data {
            // we HAVE TO call Self::write_char_unchecked
            self.write_byte_unchecked( *byte );
        }
    }


    pub fn into_ascii_string( self ) -> StdResult<AsciiString, Self> {
        if self.mail_type == MailType::Internationalized {
            Err( self )
        } else {
            //OPTIMIZE: use unchecked conversion
            //Ok( unsafe { AsciiString::from_ascii_unchecked( self.inner ) } )
            Ok( AsciiString::from_ascii( self.inner ).unwrap() )
        }
    }

}

impl MailEncoder for MailEncoderImpl {

    fn mail_type( &self ) -> MailType {
        self.mail_type
    }

    fn write_new_line( &mut self ) {
        if self.current_line_byte_length != 0 {
            self.write_char( AsciiChar::CarriageReturn );
            self.write_char( AsciiChar::LineFeed );
            self.current_line_byte_length = 0;
            self.last_cfws_pos = None;
        }
    }

    //FIXME forbid writing cfws at begin of line
    //FIXME add write_fws_with_value( c: char ) to write e.g. '\t'
    fn write_fws(&mut self ) {
        self.write_char( AsciiChar::Space );
        self.last_cfws_pos = Some( self.inner.len()-1 )
    }

    fn note_optional_fws(&mut self ) {
        self.last_cfws_pos = match self.inner.len() {
            0 => None,
            len =>  Some( len - 1 )
        };
    }

    fn break_line_on_last_cfws( &mut self )  {
        //FIXME forbid the creation of "ws-only lines in broken headers"
        if let Some( cfws_pos ) = self.last_cfws_pos {
            self.last_cfws_pos = None;

            if self.inner[cfws_pos] == b' ' {
                insert_bytes(&mut self.inner, cfws_pos, b"\r\n" );
            } else {
                insert_bytes(&mut self.inner, cfws_pos, b"\r\n " );
            }

            self.current_line_byte_length = self.inner.len() - (cfws_pos + 2)
        }
    }

    fn current_line_byte_length(&self ) -> usize {
        self.current_line_byte_length
    }

    fn write_char( &mut self, char: AsciiChar ) {
        self.write_byte_unchecked( char.as_byte() );
    }

    fn write_str( &mut self, str: &AsciiStr ) {
        self.write_data_unchecked( str.as_bytes() );
    }

    fn try_write_atext( &mut self, str: &str ) -> Result<()> {
        if word.chars().all( |ch| is_atext( ch, self.mail_type() ) ) {
            self.write_data_unchecked( str.as_bytes() );
            Ok( () )
        } else {
            bail!( "can not write atext, input is not valid atext" );
        }
    }

    fn write_encoded_word( &mut self, data: &str, ctx: EncodedWordContext ) {
        //FIXME there are two limites:
        // 1. the line length limit of 78 chars per line (including header name!)
        // 2. the quotable_string limit of 75 chars including quotings IN HEADERS ONLY (=?utf8?Q?<data>?=)
        //FIXME there are different limitations for different positions in which encoded-word appears
        self.write_str( ascii_str! {
            Equal Question u t f _8 Question Q Question
        });
        q_encode_for_encoded_word(self, ctx, data );
        self.write_str( ascii_str! {
            Question Equal
        })
    }


}


impl Into<Vec<u8>> for MailEncoderImpl {
    fn into(self) -> Vec<u8> {
        self.inner
    }
}



pub trait MailEncodable {
    fn encode<E>( &self, encoder:  &mut E ) -> Result<()>
        where E: MailEncoderImpl;
}

//pub trait MailDecodable: Sized {
//
//    //FIXME maybe &[u8]
//    fn decode( &str ) -> Result<Self>; //maybe AsRef<AsciiStr>
//
//}


#[cfg(unimplemented_test)]
mod test {

    use super::MailEncoderImpl;

    // test line length check

    macro_rules! io_encoded_word {
        ($name:ident, $inp:expr, $out:expr) => {
             io_encoded_word! { $name, false, $inp, $out }
        };
        ($name:ident, $bits8:expr, $inp:expr, $out:expr) => {
            #[test]
            fn $name() {
                let mut encoder = MailEncoder::new( $bits8 );
                encoder.write_encoded_word( $inp );
                assert_eq!(
                    $out,
                    &*encoder.to_string()
                );
            }
        }
    }

    io_encoded_word! { simple, "abcde", "=?utf8?Q?abcde?=" }
    io_encoded_word! { bracket, "<3", "=?utf8?Q?=3C3?=" }
    //length 75 of encoded_word but only in header, there is also line length 78!!
    io_encoded_word! { length_75,
        concat!(
            //50
            "the max length of a encoded word is 75 chars567890",
            "1234567890","1234567890","12345"
        ),
        concat!(
            "=?utf8?Q?the_max_length_of_a_encoded_word_is_75_chars567890",
            "1234567890",
            "1234?=", "\r\n ", "=?utf8?Q?567890",
            "1234567890","12345?=",
        )
    }

}

