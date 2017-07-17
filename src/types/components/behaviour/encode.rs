use std::char;


use ::ascii::{ AsciiChar, AsAsciiStr };

use error::*;
use codec::MailEncoder;
use codec::utf8_to_ascii::puny_code_domain;
use types::components::data_types::*;
use types::shared::Item;

pub trait EncodeComponent {
    fn encode( &self, matching_data: &Item, encoder: &mut MailEncoder ) -> Result<()>;
}

impl EncodeComponent for Domain {
    //FIXME currently does not support domain literal form
    fn encode( &self, matching_data: &Item, encoder: &mut MailEncoder ) -> Result<()> {
        let data = self.apply_on( matching_data );
        encoder.note_optional_fws();
        puny_code_domain( data, encoder );
        encoder.note_optional_fws();
        Ok( () )
    }
}

impl EncodeComponent for LocalPart {
    fn encode( &self, matching_data: &Item, encoder: &mut MailEncoder ) -> Result<()> {
        let data = self.apply_on( matching_data );
        encoder.note_optional_fws();
        encoder.try_write_8bit_data( data.as_bytes() )
            .chain_err( || ErrorKind::NonEncodableComponents( "address/addr-spec/local-part", data.into() ) )?;
        encoder.note_optional_fws();
        Ok( () )
    }
}

impl EncodeComponent for Email {
    fn encode( &self, matching_data: &Item, encoder: &mut MailEncoder ) -> Result<()> {
        self.local.encode( matching_data, encoder )?;
        encoder.write_char( AsciiChar::At );
        self.domain.encode( matching_data, encoder )?;
        Ok( () )
    }
}

impl EncodeComponent for Phrase {
    fn encode( &self, matching_data: &Item, encoder: &mut MailEncoder ) -> Result<()> {
        sep_for!{ word in self.0.iter();
            sep { encoder.write_fws() };

            word.encode( matching_data, encoder )?;
        }
        Ok( () )
    }
}

impl EncodeComponent for Word {
    fn encode( &self, matching_data: &Item, encoder: &mut MailEncoder ) -> Result<()> {
        let data = self.0.apply_on( matching_data );
        encoder.note_optional_fws();
        if data.starts_with( "\"" ) {
            //FIXME we could "unquote" the string, split it in multiple words if nessesary and then encode it
            //we can not encode quoted strings as quoting already counts as encoding
            encoder.try_write_8bit_data( data.as_bytes() )?
        } else {
            //FIXME actually there might be some ascii chars we need to escape
            if let Ok( ascii ) = data.as_ascii_str() {
                encoder.write_str( ascii );
            } else {
                //FIXME do we need to check if it's a non-ascii
                encoder.write_encoded_word( data )
            }
        }
        encoder.note_optional_fws();
        Ok( () )
    }
}

impl EncodeComponent for Address {
    fn encode( &self, matching_data: &Item, encoder: &mut MailEncoder ) -> Result<()> {
        if let Some( display_name ) = self.display_name.as_ref() {
            display_name.encode( matching_data, encoder )?;

            encoder.write_fws();
            encoder.write_char( AsciiChar::LessThan);
        }

        self.email.encode( matching_data, encoder )?;

        if self.display_name.is_some() {
            encoder.write_char( AsciiChar::GreaterThan );
        }
        Ok( () )
    }
}


impl EncodeComponent for Unstructured {
    fn encode( &self, matching_data: &Item, encoder: &mut MailEncoder ) -> Result<()> {
        //Note: the rfc 2047 does not directly state all use-cases of "unstructured" can be encoded
        // with encoded word's, but it list practically all cases unstructured can appear in
        let data = self.0.apply_on( matching_data );
        //TODO allow the data to contains thinks like '\t' etc.
        //FIXME do not replace any kind of whitespace with space
        // this is necessary if we don't want to change the type of unstructured to Vec<Rang<usize>>
        // and we also want to preserves whitespace structures in e.g. Subject lines
        // only CRLFWS should map to WS as line breaking is our think
        // not that WS are not all whitespaces, e.g. '\n' should not count into it and ascii only
        // NOTE: do not use split_whitespaces as we want to preserve the number of whitespaces
        for part in data.split( char::is_whitespace ) {
            if let Ok( ascii_part ) = part.as_ascii_str() {
                encoder.write_str( ascii_part );
            } else {
                encoder.write_encoded_word( part )
            }
            encoder.write_fws();
        }
        Ok( () )
    }
}

#[cfg(test)]
mod test {
    use codec::Bits8State;
    use types::shared::Item;
    use super::*;


    mod local {
        use super::*;

        macro_rules! test {
            ($name:ident, $bits8: expr, $data:expr, $state:expr) => {
                #[test]
                fn $name() {
                    let _data = $data;
                    let data = Item::from( _data.clone() );
                    let local_part = LocalPart( 0..data.len() );
                    let mut encoder = MailEncoder::new( $bits8 );

                    local_part.encode( &data, &mut encoder ).expect( "encoding not to fail" );

                    assert_eq!( $state, encoder.bits8_support() );
                    let encoded_bytes: Vec<u8> = encoder.into();
                    let expected_bytes: Vec<u8> = _data.into();
                    assert_eq!( expected_bytes, encoded_bytes );

                }
            }
        }

        test!{ ascii, false, "ascii_local", Bits8State::Unsupported }
        test!{ ascii_more_supported, true, "ascii_local", Bits8State::Supported }
        test!{ utf8_ok, true, "utäf8", Bits8State::Used }

        #[test]
        fn utf8_fail() {
            let data = Item::from( "utäf8" );
            let local_part = LocalPart(  0..data.len() );
            let mut encoder = MailEncoder::new( false );

            let res = local_part.encode( &data, &mut encoder );

            assert_eq!( false, res.is_ok() );
            //FIXME test if it's the correct error type
        }
    }

    #[cfg(unimplemented_tests)]
    mod domain {
        use super::*;

        macro_rules! test {
            ($name:ident, $bits8: expr, $input:expr, $output:expr) => {
                 #[test]
                fn $name() {
                    let data = Item::from( $input );
                    let domain = Domain( 0..data.len() );
                    let mut encoder = MailEncoder::new( $bits8 );
                    let bits8state = encoder.bits8_support();

                    domain.encode( &data, &mut encoder ).expect( "encoding failed" );

                    assert_eq!( bits8state, encoder.bits8_support() );
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
            let mut encoder = MailEncoder::new( false );

            email.encode( &data, &mut encoder ).expect( "encoding failed" );

            let encoded_bytes: Vec<u8> = encoder.into();
            assert_eq!( "ab@d.e", String::from_utf8_lossy( &*encoded_bytes ) );
        }
    }


    mod display_name {
        use super::*;

        #[test]
        fn mixed() {
            let data = Item::from( "Hy|ä|moin" );
            let display_name = Phrase( vec![ Word( 0..2 ), Word( 3..5 ), Word( 6..10 ) ] );
            let mut encoder = MailEncoder::new( true );

            display_name.encode( &data, &mut encoder ).expect( "encoding failed" );

            assert_eq!(Bits8State::Supported, encoder.bits8_support() );
            let encoded_bytes: Vec<u8> = encoder.into();
            assert_eq!( "Hy =?utf8?Q?=C3=A4?= moin", String::from_utf8_lossy( &*encoded_bytes ) );
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
            let mut encoder = MailEncoder::new( false );

            address.encode( &data, &mut encoder ).expect( "encoding failed" );

            let encoded_bytes: Vec<_> = encoder.into();
            assert_eq!( "ab@d.e", String::from_utf8_lossy( &*encoded_bytes ) );
        }

        #[test]
        fn with_dispaly_name() {
            let data = Item::from( "Liz|ab|d.e" );
            let address = Address {
                display_name: Some( Phrase( vec! [ Word( 0..3 ) ] ) ),
                email: Email {
                    local: LocalPart( 4..6 ),
                    domain: Domain( 7..10 ),
                }
            };

            let mut encoder = MailEncoder::new( false );

            address.encode( &data, &mut encoder ).expect( "encoding failed" );

            let encoded_bytes: Vec<_> = encoder.into();
            assert_eq!( "Liz <ab@d.e>", String::from_utf8_lossy( &*encoded_bytes ) );

        }
    }

}