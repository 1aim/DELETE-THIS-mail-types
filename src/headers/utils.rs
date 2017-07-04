use ascii::{ AsciiStr, AsciiChar };

use error::*;
use types::{ AddressList, Unstructured };
use codec::SmtpDataEncoder;

#[inline]
pub fn encode_header_key( key: &AsciiStr, encoder: &mut SmtpDataEncoder ) {
    encoder.write_str( key );
    encoder.write_char( AsciiChar::Colon );
}


pub fn encode_address_list( header_name: &AsciiStr, addr_list: &AddressList, encoder: &mut SmtpDataEncoder ) -> Result<()> {
    encode_header_key( header_name, encoder );
    // works because any form of address has a leading `[CFWS]`
    encoder.write_cfws();
    addr_list.encode( encoder )?;
    Ok( () )
}

pub fn encode_unstructured( header_name: &AsciiStr, unstructured: &Unstructured, encoder: &mut SmtpDataEncoder ) -> Result<()> {
    encode_header_key( header_name, encoder );
    unstructured.encode( encoder )
}