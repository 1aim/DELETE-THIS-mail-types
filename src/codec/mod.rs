use std::fmt;
use std::result::{ Result as StdResult };
use std::ascii::AsciiExt;

use ascii::{ AsciiString, AsciiStr, AsciiChar };

use error::*;

pub mod utf8_to_ascii;

use self::utf8_to_ascii::q_encode;

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum Utf8State {
    Unsupported,
    Supported,
    Used
}

impl Utf8State {
    pub fn is_supported( &self ) -> bool {
        use self::Utf8State::*;
        match *self {
            Supported | Used => true,
            Unsupported => false
        }
    }
    pub fn is_used( &self ) -> bool {
        match *self {
            Utf8State::Used => true,
            _ => false
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct SmtpDataEncoder {
    inner: String,
    current_line_length: usize,
    last_cfws_pos: Option<usize>,
    smtputf8_support: Utf8State
}

impl SmtpDataEncoder {
    pub fn new(smtputf8_supported: bool) -> SmtpDataEncoder {
        let smtputf8_supported = if smtputf8_supported {
            Utf8State::Supported
        } else {
            Utf8State::Unsupported
        };
        SmtpDataEncoder {
            smtputf8_support: smtputf8_supported,
            inner: String::new(),
            current_line_length: 0,
            last_cfws_pos: None
        }
    }

    // FIXME have a with_context function like e.g.: `with_context( Header, |encoder| { ... } )`
    // through this means certain switches can be modified including:
    // `max_line_length` / `preferred_max_line_length` / handling of `\r`, `\n` (especially orphan versions of them)
    // additional properties for write_encoded_word ( max word length of 75 for header )

    pub fn write_new_line(&mut self ) {
        if self.current_line_length != 0 {
            self.write_char( AsciiChar::CarriageReturn );
            self.write_char( AsciiChar::LineFeed );
            self.current_line_length = 0;
            self.last_cfws_pos = None;
        }
    }

    //FIXME forbid writing cfws at begin of line
    //FIXME add write_fws_with_value( c: char ) to write e.g. '\t'
    pub fn write_fws(&mut self ) {
        self.write_char( AsciiChar::Space );
        self.last_cfws_pos = Some( self.inner.len()-1 )
    }

    pub fn note_optional_fws(&mut self ) {
        self.last_cfws_pos = match self.inner.len() {
            0 => None,
            len =>  Some( len - 1 )
        };
    }

    pub fn write_str( &mut self, str: &AsciiStr ) {
        self.write_str_unchecked( str.as_str() );
    }

    pub fn write_char( &mut self, char: AsciiChar ) {
        self.write_char_unchecked( char.as_char() );
    }

    pub fn try_write_utf8_str( &mut self, str: &str ) -> Result<()> {
        use self::Utf8State::*;
        match self.smtputf8_support {
            Unsupported if !str.is_ascii() =>
                return Err( ErrorKind::TriedWritingUtf8IntoAsciiData.into() ),
            Supported if !str.is_ascii() =>
                self.smtputf8_support = Used,
            _ => {}
        }
        self.write_str_unchecked( str );
        Ok( () )
    }

    fn write_char_unchecked( &mut self, char: char ) {
        //FIXME: potentially keep track of "line ending state" to prevent rogue '\r' '\n'
        //THIS IS THE ONLY FUNCTION WHICH SHOULD WRITE TO self.inner!!
        self.inner.push( char );
        self.current_line_length += 1;
        if char == '\n' && self.inner.chars().last().unwrap() == '\r' {
            self.current_line_length = 0;
            self.last_cfws_pos = None;
        }
    }

    fn write_str_unchecked(&mut self, str: &str ) {
        for char in str.chars() {
            // we HAVE TO call Self::write_char_unchecked
            self.write_char_unchecked( char );
        }
    }

    //we want to encode < for
    pub fn write_encoded_word( &mut self, data: &str ) {
        //FIXME there are two limites:
        // 1. the line length limit of 78 chars per line (including header name!)
        // 2. the quotable_string limit of 75 chars including quotings IN HEADERS ONLY (=?utf8?Q?<data>?=)
        //FIXME there are different limitations for different positions in which encoded-word appears
        self.write_str( ascii_str! {
            Equal Question u t f _8 Question Q Question
        });
        q_encode( data, self );
        self.write_str( ascii_str! {
            Question Equal
        })
    }

    pub fn break_line_on_last_cfws( &mut self )  {
        //FIXME forbid the creation of "ws-only lines in broken headers"
        if let Some( cfws_pos ) = self.last_cfws_pos {
            self.last_cfws_pos = None;

            if self.inner.as_bytes()[cfws_pos] == 0x20 {
                self.inner.insert_str( cfws_pos, "\r\n" );
            } else {
                self.inner.insert_str( cfws_pos, "\r\n " );
            }

            // could be utf8 under some circumstances
            if self.smtputf8_support.is_used() {
                self.current_line_length = self.inner[cfws_pos+2..].chars().count();
            } else {
                self.current_line_length = self.inner.len() - (cfws_pos+2)
            }
        }
    }

    pub fn current_line_length( &self ) -> usize {
        self.current_line_length
    }

    pub fn smtputf8_support( &self ) -> Utf8State {
        self.smtputf8_support
    }

    pub fn into_ascii_string( self ) -> StdResult<AsciiString, Self> {
        if self.smtputf8_support.is_used() {
            Err( self )
        } else {
            Ok( unsafe { AsciiString::from_ascii_unchecked( self.inner ) } )
        }
    }

}



impl Into<String> for SmtpDataEncoder {
    fn into(self) -> String {
        self.inner
    }
}


impl fmt::Display for SmtpDataEncoder {

    fn fmt( &self, f: &mut fmt::Formatter ) -> fmt::Result {
        write!( f, "{}", self.inner.as_str() )
    }
}


pub trait SmtpDataEncodable {

    fn encode( &self, &mut SmtpDataEncoder ) -> Result<()>; //possible Cow later on
}

pub trait SmtpDataDecodable: Sized {

    //FIXME maybe &[u8]
    fn decode( &str ) -> Result<Self>; //maybe AsRef<AsciiStr>

}


#[cfg(unimplemented_test)]
mod test {

    use super::SmtpDataEncoder;

    // test line length check

    macro_rules! io_encoded_word {
        ($name:ident, $inp:expr, $out:expr) => {
             io_encoded_word! { $name, false, $inp, $out }
        };
        ($name:ident, $utf8:expr, $inp:expr, $out:expr) => {
            #[test]
            fn $name() {
                let mut encoder = SmtpDataEncoder::new( $utf8 );
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

