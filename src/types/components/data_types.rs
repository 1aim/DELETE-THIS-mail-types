

use std::ops::Range;

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct Unstructured( Range<usize> );

#[derive(Debug,  Clone, Hash, PartialEq, Eq)]
pub struct Address {
    pub display_name: Option<DisplayName>,
    pub email: Email
}

///
/// Note: the vector in DisplayName SHOULD not be empty, use option to express optionality
#[derive(Debug,  Clone, Hash, PartialEq, Eq)]
pub struct DisplayName( pub Vec<Range<usize>> );

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

impl View for DisplayName {
    fn apply_on<'s,'out>( &'s self, matching_data: &'out str ) -> &'out str {
        match self.0.len() {
            0 => "",
            x => &matching_data[Range { start: self.0[0].start, end: self.0[x-1].end } ]
        }
    }
}

impl View for Address {
    fn apply_on<'s,'out>( &'s self, matching_data: &'out str ) -> &'out str {
        let mut start = self.email.local.0.start;
        let mut end = self.email.domain.0.end;
        if let Some( display_name ) = self.display_name.as_ref() {
            if let Some( first ) = display_name.0.first() {
                start = first.start;
                // include trailing ">"
                end += 1;
            }
        }
        &matching_data[Range { start, end }]
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
        let disp = DisplayName( vec![ 5..7, 9..10, 11..13 ] );
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
            display_name: Some( DisplayName( vec![ 5..7, 9..10 ] ) ),
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


}
