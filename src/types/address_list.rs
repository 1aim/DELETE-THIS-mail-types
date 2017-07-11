use std::ops::{ Deref, DerefMut };

use error::*;
use ascii::AsciiChar;
use codec::{ SmtpDataEncoder, SmtpDataDecodable, SmtpDataEncodable };

use super::address::Address;

pub struct OptAddressList( Vec<Address> );

pub struct AddressList( OptAddressList );

impl OptAddressList {
    pub fn new( list: Vec<Address> ) -> Self {
        OptAddressList( list )
    }
}

impl AddressList {

    pub fn new_with_first( first: Address ) -> Self {
        AddressList( OptAddressList( vec![ first ] ) )
    }

    pub fn new( list: Vec<Address> ) -> Result<Self> {
        if list.is_empty() {
            Err( ErrorKind::AtLastOneElementIsRequired.into() )
        } else {
            Ok( AddressList( OptAddressList::new( list ) ) )
        }
    }

    pub fn push( &mut self, addr: Address ) {
        self.0.push( addr )
    }

    //FIXME expose more mutable non shrinking operations
}

impl Deref for OptAddressList {
    type Target = Vec<Address>;

    fn deref( &self ) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for OptAddressList {

    fn deref_mut( &mut self ) -> &mut Self::Target {
        &mut self.0
    }
}

impl Deref for AddressList {
    type Target = Vec<Address>;

    fn deref( &self ) -> &Self::Target {
        &*self.0
    }
}



impl SmtpDataEncodable for OptAddressList {
    fn encode( &self, encoder: &mut SmtpDataEncoder ) -> Result<()> {
        sep_for!{ address in self.0.iter();
            sep {
                encoder.write_char( AsciiChar::Comma );
                encoder.write_fws();
            };
            address.encode( encoder )?;
        }
        Ok( () )
    }
}

impl SmtpDataEncodable for AddressList {
    fn encode( &self, encoder: &mut SmtpDataEncoder ) -> Result<()> {
        self.0.encode( encoder )
    }
}

impl SmtpDataDecodable for AddressList {
    fn decode( data: &str ) -> Result<Self> {
        unimplemented!();
    }
}

#[cfg(test)]
mod test {
    use super::*;

    mod encode {
        use super::*;

        fn parse( s: &str ) -> Address {
            unimplemented!()
        }

        macro_rules! test {
            ($name:ident, [$($addr:expr),*], $output:expr) => {
                #[test]
                fn $name() {
                    let list = AddressList::new( vec![ $($addr),* ] ).unwrap();
                    let mut encoder = SmtpDataEncoder::new( false );
                    list.encode( &mut encoder ).expect( "encoding failed" );
                    assert_eq!( $output, encoder.to_string() );
                }
            }
        }

        //FIXME empty should err
//        test!{ empty,
//            [], "" }

        test!{ one,
            [ parse( "X <a@b.d>" ) ],
            "X <a@b.d>" }

        test!{ multiple,
            [ parse( "X <a@b.d>" ), parse( "e@d.e" ), parse( "xe@de.de" ) ],
            "X <a@b.d>, e@d.e, xe@de.de" }


    }
}