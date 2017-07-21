use std::borrow::{ Cow, ToOwned };

use ascii::{ AsciiStr, AsciiChar };

use error::*;
use types::{
    Address, OptAddressList, AddressList,
    MessageID, MessageIDList,
    Unstructured, DateTime,
    Path, ReceivedToken,
    TransferEncoding, Text,
    Mime, PhraseList, HeaderName

};

use codec::{  MailEncoder, MailEncodable };


include! { concat!( env!( "OUT_DIR" ), "/header_enum.rs.partial" )  }

//FIXME tendentially merge with types::HeaderName to some extend
pub enum HeaderNameRef<'a> {
    Static( &'static AsciiStr ),
    Other( &'a AsciiStr )
}

impl<'a> Into<Cow<'static, AsciiStr>> for HeaderNameRef<'a> {
    fn into( self ) -> Cow<'static, AsciiStr> {
        use self::HeaderNameRef::*;
        match self {
            Static( static_ref ) => Cow::Borrowed( static_ref ),
            Other( non_static_ref ) => Cow::Owned( non_static_ref.to_owned() )
        }
    }
}

impl Header {

    pub fn name<'a>( &'a self ) -> HeaderNameRef<'a> {
        use self::Header::*;
        //a match with arms like `Date( .. ) => unsafe { AsciiStr::from_ascii_unchecked( "Date" ) }`
        let fn_impl = include! { concat!( env!( "OUT_DIR" ), "/header_enum_names.rs.partial" )  };
        fn_impl( self )
    }
}


impl MailEncodable for Header {

    fn encode( &self, encoder: &mut MailEncoder ) -> Result<()> {
        use self::Header::*;
        //a match with arms like: `Date( ref field ) => encoder_header_helper( "Date", field, encoder ),`
        let fn_impl = include!( concat!( env!( "OUT_DIR", ), "/encoder_match_cases.rs.partial" ) );
        fn_impl( self, encoder )
    }
}

fn encode_header_helper<T: MailEncodable>(
    name: &AsciiStr, encodable: &T, encoder: &mut MailEncoder
) -> Result<()> {
    encoder.write_str( name );
    encoder.write_char( AsciiChar::Colon );
    //any of the following types have a leading [CFWS] so we just "write" it out here
    //NOTE: for some data like text/unstructured the space theoretically belongs to the data
    encoder.write_fws();
    encodable.encode( encoder )
}



