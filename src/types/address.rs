use std::fmt;
use ascii::AsciiStr;

use error::*;
use types::shared::Item;
use codec::{MailDecodable, MailEncodable, MailEncoder};


use types::components::data_types;
use types::components::data_types::View;
use types::components::behaviour::encode::EncodeComponent;


pub struct Address {
    inner: Item,
    component_slices: data_types::Address,
    // give the parser some slack for some non valide but nevertheless occurring syntax, this
    // include thing parsed through `obs-` grammar which can't be converted to valid grammar
    // e.g. display-name-only/user-name-only style (e.g. `From: admin` )
    //valid: bool
}


impl Address {

    pub fn display_name( &self ) -> Option<&str> {
        self.component_slices.display_name.as_ref().map( |dn| {
            dn.apply_on( &*self.inner )
        })
    }

    pub fn user( &self ) -> &str {
        self.component_slices.email.local.apply_on( &*self.inner )
    }

    // is required both in actual and obsolete syntax specified in RFC
    pub fn host( &self ) -> &str {
        self.component_slices.email.domain.apply_on( &*self.inner )
    }


}

impl MailEncodable for Address {

    fn encode( &self, encoder: &mut MailEncoder ) -> Result<()> {
        self.component_slices.encode( &self.inner, encoder )
    }
}

impl MailDecodable for Address {

    fn decode( _src: &str ) -> Result<Self> {
        unimplemented!();
    }
}



impl fmt::Debug for Address {
    //use encode and convert to String
    fn fmt( &self, f: &mut fmt::Formatter ) -> fmt::Result {
        let mut encoder = MailEncoder::new( true );
        if let Err(_) = self.encode( &mut encoder ) {
            //FIXME warn!
            return Err( fmt::Error )
        }
        let encoded_bytes: Vec<_> = encoder.into();
        write!( f, "{}", String::from_utf8_lossy( &*encoded_bytes ) )
    }
}


#[cfg(unimplemented_test)]
mod test {
    use std::ops::Range;
    use std::rc::Rc;

    use owning_ref::OwningRef;
    use ascii::AsAsciiStr;

    use super::super::components::{ data_types as address };
    use codec::{ MailDecodable, MailEncodable, MailEncoder };
    use super::Address;

    macro_rules! check_addr {
        (
            $inp:expr,
            user $user:expr,
            host $host:expr,
            name $($name:expr),*
        ) => { #[allow(unused_mut)] {
            let value = $inp;
            assert_eq!( value.user(), $user );
            assert_eq!( value.host(), $host );
            let mut out = String::new();
            $( out.push_str( $name ); out.push( ' ' ); )*

            let out = if out.len() > 0 {
                Some( out.trim() )
            } else {
                None
            };

            assert_eq!(
                out,
                value.display_name()
            );
        }}
    }

    fn _range_push( to: &mut String, value: &str ) -> Range<usize> {
        let start = to.len();
        to.push_str( value );
        let end = to.len();
        Range { start, end }

    }

    macro_rules! addr {
        (
            user $user:expr,
            host $host:expr,
            name $($name:expr),*
        ) => {{
            let mut buf = String::new();
            let mut name_ranges = Vec::new();
            $(
                name_ranges.push( _range_push( &mut buf, $name ) );
                buf.push( ' ' );
            )*

            let name_ranges = if name_ranges.len() > 0 {
                Some( name_ranges )
            } else {
                None
            };

            buf.push('<');
            let user_range = _range_push( &mut buf, $user );
            buf.push('@');
            let host_range = _range_push( &mut buf, $host );
            buf.push('>');


            Address {
                inner: OwningRef::new( Rc::new( buf ) ).map( |x| &**x ),
                component_slices: address::Address {
                    display_name: name_ranges.map( |ranges| address::DisplayName( ranges ) ),
                    email: address::Email {
                        local: address::LocalPart( user_range ),
                        domain: address::Domain( host_range ),
                    }
                }
            }
        }}
    }

    macro_rules! test_encode {
        ($tname:ident, $input:expr, $result:expr) => {
            test_encode!{ $tname, false, $input, $result }
        };
        ($tname:ident, $bits8:expr, $input:expr, $result:expr) => {
            #[test]
            fn $tname() {
                let address: Address = $input;
                let mut encoder = MailEncoder::new( $bits8 );
                address.encode( &mut encoder ).unwrap();

                assert_eq!(
                    $result,
                    encoder.to_string()
                );
            }
        }
    }

    macro_rules! test_decode {
        ($tname:ident, $input:expr, $check:ident, $($tofwd:tt)*) => {
            #[test]
            fn $tname() {
                let input = $input.as_ascii_str()
                    .expect("failed converting input to ascii str");
                let addr = Address::decode( input ).expect( "failed decoding" );
                $check! { addr, $($tofwd)* }
            }
        }
    }

    test_decode!( normal, r#"culone <culo@culetto>"#, check_addr,
        user "culo",
        host "culetto",
        name "culone"
    );

    test_decode!( normal2, r#"Max Musterman <ma.x@muster.man>"#, check_addr,
        user "ma.x",
        host "muster.man",
        name "Max", "Musterman"
    );

    test_decode!(normal_quoted, r#""culone" <culo@culetto>"#, check_addr,
        user "culo",
        host "culetto",
        name "culone"
    );

    test_decode!( only_address, r#"culo@culetto"#, check_addr,
        user "culo",
        host "culetto",
        name
    );

    test_decode!( only_address_brackets, r#"<culo@culetto>"#, check_addr,
        user "culo",
        host "culetto",
        name
    );

    //FIXME this is not valid in the standard but does appear nevertheless
    // support it as a "display name only" style, through it could
    // also be a user name only style we never know if it's it
    // ALSO DO NOT SUPPORT GENERATING IT, it's not valid at all
//    test_decode!(only_user, r#"culo"#, check_addr,
//        name None,
//        user Some("culo"),
//        host None
//    );

    test_encode!( utf8_in_name, addr! {
        user "oa",
        host "o.a",
        name "aö"
    }, "=?utf8?Q?a=C3=B6?= <oa@o.a>" );

    test_encode!( utf8_in_name2, addr! {
        user "oa",
        host "o.a",
        name "abc aö öa"
    }, "abc =?utf8?Q?a=C3=B6?= =?utf8?Q?=C3=B6a?= <oa@o.a>" );

    //test that "<3 <some@thing>" does not work
    //TODO more encoding tests
    //TODO more tests using assert_eq instead of check_addr?!
}




