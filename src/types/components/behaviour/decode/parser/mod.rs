use std::ops::Range;
use nom::{ traits as nom_traits };

use types::components::data_types::*;
use self::slice::Slice;

#[macro_use]
mod utils;

mod slice;


my_names!( fws, //obs-fws
    recognize!(
        do_parse!(
            opt!( do_parse!(
                take_while!( is_ws ) >>
                char!( '\r' ) >>
                char!( '\n' ) >>
                ()
            ) ) >>
            take_while1!( is_ws ) >>
            ()
        )
    )
);

my_named!( comment< Slice >,
    delimited!(
        char!( '(' ),
        recognize!( postceded!(
            many0!( preceded! (
                opt!( fws ),
                alt!(
                    verify_char( |ch| is_ctext ) => { void() } |
                    quoted_pair =>  { void() } |
                    comment => { void() }
                )
            ) ),
            opt!( fws )
        ) ),
        char!( ')' )
    )
);

my_named!( quoted_pair,
    preceded!(
        char!( '\\' ),
        verify_char!( |ch| ch = ' ' || is_vchar( ch ) )
    )
);

my_named!( cfws< Vec< Slice > >,
    alt!(
        fws => { |_| vec![] } |
        postceded!(
            many1( preceding!( opt!( fws ), comment ) ),
            opt!( fws )
        )
    )
);

my_named!( dot_atom_text,
    recognize!( tuple!(
        take_while1!( is_atext ),
        many0!( preceding!(
            char!( '.' ),
            take_while1!( is_atext )
        ) )
    ) )
);

my_named!( dot_atom,
    delimited!(
        opt!( cfws ),
        dot_atom_text,
        opt!( cfws )
    )
);



my_named!( quoted_string,
    delimited!(
        opt!( cfws ),
        recognize!( tuple!(
            char!( '"' ),
            many0!(
                preceded!(
                    opt!( fws ),
                    alt!(
                        quoted_pair => { void() } |
                        take_while1!( is_qtext ) => { void() }
                    )
                )
            ),
            opt!( fws ),
            char!( '"' )
        ) ),
        opt!( cfws )
    )
);


//alt!( ... | dot_atom | domain_literal | obs-domain ) );
my_named!( domain< Domain >,
    map!(
        dot_atom,
        |slice| {
            Domain( slice.as_base_range() )
        }
    )
);

my_named!( local_part< LocalPart >,
    map!(
        alt!( dot_atom | quoted_string ) //| obs_local_part )),
        |slice| {
            LocalPart( slice.as_base_range() )
        }
    )
);

my_named!( email< Email >,
    do_parse!(
        loc: local_part >>
        char!( '@' ) >>
        dom: domain >>
        (
            Email {
                local: loc,
                domain: dom
            }
        )
    )
);

my_named!( named_address< Address >,
    do_parse!(
        dname: opt!( display_name ) >>
        opt!( cfws ) >>
        char!( '<' ) >>
        addr: email >>
        char!( '>' ) >>
        opt!( cfws ) >>

        ( Address { display_name: dname, email: addr } )
    )
);

my_named!( mailbox< Address >,
    alt!(
        complete!( named_address ) => { |addr| addr } |
        email => { |email| Address { email, display_name: None } }
        //FIXME add user only fallback e.g. email -> email<(Option<&[u8]>, &[u8])>
        // | local_part => {  |user| ?? }
    )
);

my_named!( mailbox_list< Vec< Address > >,
    do_parse!(
        first: mailbox >>
        res: fold_many0!(
            do_parse!( char!(',') >> addr: mailbox >> (addr) ),
            vec![ first ],
            | mut list: AddressList, item | {
                list.push( item );
                list
            }
        ) >>
        ( res )
    )
);


my_named!( atom,
    delimited!(
        opt!( cfws ),
        take_while1!( is_atext ),
        opt!( cfws )
    )
);

my_named!( word< Word >,
    map!(
        alt!( atom | quoted_string )
        |slice| {
            Word( slice.as_base_range() )
        }
    )
);

my_named!( phrase< Phrase >, //ops-phrase
    map!(
        many1( word ),
        |vec| Phrase( vec )
    )
);

my_named!( unstructured< Unstructured >, //ops-unstructured
    map!(
        recognize!(
            take_while!( |x| is_ws(x) || is_vchar(x) )
        ),
        |text| Unstructured( text.as_base_range() )
    )
);






