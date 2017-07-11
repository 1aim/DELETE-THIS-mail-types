use ascii::{ AsciiString, AsciiStr };

use std::ops::Deref;
use error::*;
use codec::{ MailEncoder, MailEncodable };

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub enum TransferEncoding {
    _7Bit,
    _8Bit,
    Binary,
    QuotedPrintable,
    Base64,
    // should be only ietf-token (i.e. tokens standarized through an RFC and registered with IANA)
    // but we don't check this so it's other and not ietf token
    //FIXME not sure if the limitations are to tight (with Token)
    //FIXME allow puting XTokens into OtherToken when generating?
    Other( Token ),
}

impl MailEncodable for TransferEncoding {
    fn encode( &self, encoder: &mut MailEncoder ) -> Result<()> {
        use self::TransferEncoding::*;
        match *self {
            _7Bit => encoder.write_str( ascii_str! { _7 b i t } ),
            _8Bit => encoder.write_str( ascii_str! { _8 b i t } ),
            Binary => encoder.write_str( ascii_str! { b i n a r y } ),
            QuotedPrintable => encoder.write_str( ascii_str! { q u o t e d Minus p r i n t a b l e } ),
            Base64 => encoder.write_str( ascii_str! { b a s e _6 _4 } ),
            Other( ref token ) => token.encode( encoder )?
        }
        Ok( () )
    }
}


//FIXME limit chars valid for token (no space, no special chars like {([" ... )
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct Token( AsciiString );

impl Token {

    fn is_x_token( &self ) -> bool {
        let bytes = self.as_bytes();
        bytes[1] == b'-' && ( bytes[0] == b'X' || bytes[0] == b'x' )
    }
}

impl MailEncodable for Token {

    fn encode( &self, encoder: &mut MailEncoder ) -> Result<()> {
        encoder.write_str( &*self.0 );
        Ok( () )
    }

}

impl  Deref for Token {
    type Target = AsciiStr;
    fn deref( &self ) -> &AsciiStr {
        &*self.0
    }
}