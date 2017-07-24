

use std::ops::Range;

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct Word( pub Range<usize> );

//FIXME when parsing make sure no controll character do appear
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct Unstructured( pub Range<usize> );

#[derive(Debug,  Clone, Hash, PartialEq, Eq)]
pub struct Address {
    pub display_name: Option<Phrase>,
    pub email: Email
}

//TODO crate a VecGt1 vector with minimal length 1! (new(first), pop last fails etc.)
// also use this for some of the other 1*xxx parts
#[derive(Debug,  Clone, Hash, PartialEq, Eq)]
pub struct Phrase(pub Vec<Word> );

#[derive(Debug,  Clone, Hash, PartialEq, Eq)]
pub struct Email { pub local: LocalPart, pub domain: Domain }

#[derive(Debug,  Clone, Hash, PartialEq, Eq)]
pub struct LocalPart( pub Range<usize> );

#[derive(Debug,  Clone, Hash, PartialEq, Eq)]
pub struct Domain( pub Range<usize> );

pub trait View {
    fn apply_on<'s,'out>( &'s self, matching_data: &'out str ) -> &'out str;
}

impl View for Range<usize> {
    fn apply_on<'s,'out>( &'s self, matching_data: &'out str ) -> &'out str {
        &matching_data[self.clone()]
    }
}
impl View for Domain {
    fn apply_on<'s,'out>( &'s self, matching_data: &'out str ) -> &'out str {
        self.0.apply_on( matching_data )
    }
}

impl View for LocalPart {
    fn apply_on<'s,'out>( &'s self, matching_data: &'out str ) -> &'out str {
        self.0.apply_on( matching_data )
    }
}

impl View for Email {
    fn apply_on<'s,'out>( &'s self, matching_data: &'out str ) -> &'out str {
        &matching_data[Range { start: self.local.0.start, end: self.domain.0.end }]
    }
}

impl View for Phrase {
    fn apply_on<'s,'out>( &'s self, matching_data: &'out str ) -> &'out str {
        match self.0.len() {
            0 => "",
            x => &matching_data[Range { start: self.0[0].0.start, end: self.0[x-1].0.end } ]
        }
    }
}

impl View for Address {
    fn apply_on<'s,'out>( &'s self, matching_data: &'out str ) -> &'out str {
        let mut start = self.email.local.0.start;
        let mut end = self.email.domain.0.end;
        if let Some( display_name ) = self.display_name.as_ref() {
            if let Some( first ) = display_name.0.first() {
                start = first.0.start;
                // include trailing ">"
                end += 1;
            }
        }
        &matching_data[Range { start, end }]
    }
}

impl View for Unstructured {
    fn apply_on<'s, 'out>( &'s self, matching_data: &'out str ) -> &'out str {
        self.0.apply_on( matching_data )
    }
}

impl View for Word {
    fn apply_on<'s, 'out>( &'s self, matching_data: &'out str ) -> &'out str {
        self.0.apply_on( matching_data )
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn domain_view() {
        let domain = Domain( Range { start: 1, end: 4 } );
        assert_eq!(
            "bcd",
            domain.apply_on( "abcde" )
        );
    }

    #[test]
    fn local_part_view() {
        let local_part = LocalPart( Range { start: 1, end: 4 });
        assert_eq!(
            "bcd",
            local_part.apply_on( "abcde" )
        );
    }

    #[test]
    fn email_view() {
        let email = Email {
            local: LocalPart( 4..7 ),
            domain: Domain( 8..11 )
        };
        assert_eq!(
            "bcd@e.f",
            email.apply_on( "Ha <bcd@e.f>" )
        );
    }

    #[test]
    fn display_name_view() {
        let disp = Phrase( vec![ Word( 5..7 ), Word( 9..10 ), Word( 11..13 ) ] );
        assert_eq!(
            "ab cd ef",
            disp.apply_on("Bcc: ab cd ef <q@e.f>")
        )
    }

    #[test]
    fn address_view_without_display_name() {
        let addr = Address {
            display_name: None,
            email: Email {
                local: LocalPart( 1..3 ),
                domain: Domain( 4..7 )
            }
        };
        assert_eq!(
            "ab@c.e",
            addr.apply_on( "<ab@c.e>" )
        );
    }

    #[test]
    fn address_view_with_display_name() {
        let addr = Address {
            display_name: Some( Phrase( vec![ Word( 5..7 ), Word( 9..10 ) ] ) ),
            email: Email {
                local: LocalPart( 12..14 ),
                domain: Domain( 15..18 )
            }
        };
        assert_eq!(
            "Ha  A <ab@c.e>",
            addr.apply_on( "Bcc: Ha  A <ab@c.e>" )
        );
    }


    #[test]
    fn unstructured_name_view() {
        let text = "Subject: This is fun";
        let disp = Unstructured( 9..text.len() );
        assert_eq!(
            "This is fun",
            disp.apply_on( text )
        )
    }

    #[test]
    fn word_view() {
        let text = " Abc ";
        let disp = Word( 1..4 );
        assert_eq!(
            "Abc",
            disp.apply_on( text )
        )
    }



}
