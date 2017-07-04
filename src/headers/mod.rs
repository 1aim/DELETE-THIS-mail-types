use mime::Mime;
use ascii::{ AsciiString, AsciiStr };

use error::*;
use types::{
    Address, OptAddressList, AddressList,
    MessageID, MessageIDList,
    Unstructured, DateTime
};

use codec::{ SmtpDataDecodable, SmtpDataEncoder, SmtpDataEncodable };

use self::utils::{ encode_address_list, encode_unstructured };

mod utils;

// name -> Name( _ ) => "Name"
// parse -> "Name" => parse(...)
// encode -> name().encoder( enc ); &&+ Name( ref x ) => { x.encode( enc ) }


include! { concat!( env!( "OUT_DIR" ), "/header_enum.rs.partial" )  }




impl SmtpDataEncodable for Header {

    fn encode( &self, encoder: &mut SmtpDataEncoder ) -> Result<()> {
        use self::Header::*;
        let match_fn = include!( concat!( env!( "OUT_DIR", ), "/encoder_match_cases.rs.partial" ) );
        match_fn( self, encoder )
    }
}


