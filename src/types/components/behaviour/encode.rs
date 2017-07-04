use ::ascii::{ AsciiChar, AsAsciiStr };



use  types::components::data_types::*;

use error::*;
use types::shared::Item;
use codec::SmtpDataEncoder;
use codec::utf8_to_ascii::puny_code_domain;

pub trait EncodeComponent {
    fn encode( &self, matching_data: &Item, encoder: &mut SmtpDataEncoder ) -> Result<()>;
}

impl EncodeComponent for Domain {
    //FIXME currently does not support domain literal form
    fn encode( &self, matching_data: &Item, encoder: &mut SmtpDataEncoder ) -> Result<()> {
        let data = self.apply_on( matching_data );
        puny_code_domain(data, encoder );
        Ok( () )
    }
}

impl EncodeComponent for LocalPart {
    fn encode( &self, matching_data: &Item, encoder: &mut SmtpDataEncoder ) -> Result<()> {
        let data = self.apply_on( matching_data );
        encoder.try_write_utf8_str( data )
            .chain_err( || ErrorKind::NonEncodableComponents( "address/addr-spec/local-part", data.into() ) )
    }
}

impl EncodeComponent for Email {
    fn encode( &self, matching_data: &Item, encoder: &mut SmtpDataEncoder ) -> Result<()> {
        self.local.encode( matching_data, encoder )?;
        encoder.write_char( AsciiChar::At );
        self.domain.encode( matching_data, encoder )?;
        Ok( () )
    }
}

impl EncodeComponent for DisplayName {
    fn encode( &self, matching_data: &Item, encoder: &mut SmtpDataEncoder ) -> Result<()> {
        let mut first = true;
        for word in self.0.iter() {
            if first { first = false; }
                else {
                    //display-name is = phrase = 1*word = atom = [CFWS] *atext [CFWS]
                    encoder.write_cfws();
                }

            let data = word.apply_on( matching_data );
            //FIXME actually there might be some ascii chars we need to escape
            if let Ok( ascii ) = data.as_ascii_str() {
                encoder.write_str( ascii );
            } else {
                encoder.write_encoded_word( data )
            }
        }
        Ok( () )
    }
}

impl EncodeComponent for Address {
    fn encode( &self, matching_data: &Item, encoder: &mut SmtpDataEncoder ) -> Result<()> {
        if let Some( display_name ) = self.display_name.as_ref() {
            display_name.encode( matching_data, encoder )?;

            encoder.write_cfws();
            encoder.write_char( AsciiChar::LessThan);
        }

        self.email.encode( matching_data, encoder )?;

        if self.display_name.is_some() {
            encoder.write_char( AsciiChar::GreaterThan );
        }
        Ok( () )
    }
}

#[cfg(test)]
mod test {
    use codec::Utf8State;
    use types::shared::Item;
    use super::*;


    mod local {
        use super::*;

        macro_rules! test {
            ($name:ident, $utf8: expr, $data:expr, $state:expr) => {
                #[test]
                fn $name() {
                    let _data = $data;
                    let data = Item::from( _data.clone() );
                    let local_part = LocalPart( 0..data.len() );
                    let mut encoder = SmtpDataEncoder::new( $utf8 );

                    local_part.encode( &data, &mut encoder ).expect( "encoding not to fail" );

                    assert_eq!( _data, encoder.to_string() );
                    assert_eq!( $state, encoder.smtputf8_support() )

                }
            }
        }

        test!{ ascii, false, "ascii_local", Utf8State::Unsupported }
        test!{ ascii_more_supported, true, "ascii_local", Utf8State::Supported }
        test!{ utf8_ok, true, "utäf8", Utf8State::Used }

        #[test]
        fn utf8_fail() {
            let data = Item::from( "utäf8" );
            let local_part = LocalPart(  0..data.len() );
            let mut encoder = SmtpDataEncoder::new( false );

            let res = local_part.encode( &data, &mut encoder );

            assert_eq!( false, res.is_ok() );
            //FIXME test if it's the correct error type
        }
    }

    #[cfg(unimplemented_tests)]
    mod domain {
        use super::*;

        macro_rules! test {
            ($name:ident, $utf8: expr, $input:expr, $output:expr) => {
                 #[test]
                fn $name() {
                    let data = Item::from( $input );
                    let domain = Domain( 0..data.len() );
                    let mut encoder = SmtpDataEncoder::new( $utf8 );
                    let utf8state = encoder.smtputf8_support();

                    domain.encode( &data, &mut encoder ).expect( "encoding failed" );

                    assert_eq!( utf8state, encoder.smtputf8_support() );
                    assert_eq!( $output, encoder.to_string() )
                }
            }
        }

        test!{ ascii, false, "domain", "domain" }
        test!{ ascii_2, true, "domain", "domain" }
        test!{ utf8, false, "äöü", "xn--4ca0bs" }
        test!{ utf8_2, true, "äöü", "xn--4ca0bs" }

    }


    mod email {
        use super::*;

        #[test]
        fn simple() {
            let data = Item::from( "ab|d.e" );
            let email = Email {
                local: LocalPart( 0..2 ),
                domain: Domain( 3..6 )
            };
            let mut encoder = SmtpDataEncoder::new( false );

            email.encode( &data, &mut encoder ).expect( "encoding failed" );

            assert_eq!( "ab@d.e", encoder.to_string() )
        }
    }


    mod display_name {
        use super::*;

        #[test]
        fn mixed() {
            let data = Item::from( "Hy|ä|moin" );
            let display_name = DisplayName( vec![ 0..2, 3..5, 6..10 ] );
            let mut encoder = SmtpDataEncoder::new( true );

            display_name.encode( &data, &mut encoder ).expect( "encoding failed" );

            assert_eq!( "Hy =?utf8?Q?=C3=A4?= moin", encoder.to_string() );
            assert_eq!( Utf8State::Supported, encoder.smtputf8_support() );
        }
    }

    mod address {
        use super::*;

        #[test]
        fn no_display_name() {
            let data = Item::from( "ab|d.e" );
            let address = Address {
                display_name: None,
                email: Email {
                    local: LocalPart( 0..2 ),
                    domain: Domain( 3..6 ),
                }
            };
            let mut encoder = SmtpDataEncoder::new( false );

            address.encode( &data, &mut encoder ).expect( "encoding failed" );

            assert_eq!( "ab@d.e", encoder.to_string() );
        }

        #[test]
        fn with_dispaly_name() {
            let data = Item::from( "Liz|ab|d.e" );
            let address = Address {
                display_name: Some( DisplayName( vec! [ 0..3 ] ) ),
                email: Email {
                    local: LocalPart( 4..6 ),
                    domain: Domain( 7..10 ),
                }
            };

            let mut encoder = SmtpDataEncoder::new( false );

            address.encode( &data, &mut encoder ).expect( "encoding failed" );

            assert_eq!( "Liz <ab@d.e>", encoder.to_string() );

        }
    }

}