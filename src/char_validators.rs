use error::*;


//TODO move MailType to types?
pub enum MailType {
    Ascii,
    Internationalized
}

impl MailType {
    fn supports_utf8( &self ) -> bool {
        use self::MailType::*;
        match *self {
            Ascii => false,
            Internationalized => true
        }
    }
}
///WS as defined by RFC 5234
#[inline(always)]
pub fn is_ws( ch: char ) -> bool {
    // is not limited to ascii ws
    //ch.is_whitespace()
    //WSP            =  SP / HTAB
    ch == ' ' || ch == '\t'
}

#[inline(always)]
pub fn is_space( ch: char ) -> bool {
    ch == ' '
}

#[inline(always)]
pub fn is_ascii_vchar( ch: char ) -> bool {
    let u32_ch = ch as u32;
    32 < ch && ch < 128
}

//VCHAR as defined by RFC 5243
#[inline(always)]
pub fn is_vchar( ch: char, tp: MailType ) -> bool {
    use self::MailType::*;
    match tp {
        Ascii => is_ascii_vchar( ch ),
        Internationalized => is_ascii_vchar( ch ) || ch.len_utf8() > 1
    }
}

///any whitespace (char::is_whitespace
#[inline(always)]
pub fn is_any_whitespace(ch: char) -> bool {
    ch.is_whitespace()
}

//ctext as defined by RFC 5322
pub fn is_ctext( ch: char, tp: MailType  ) -> bool {
    match ch {
        '!'...'\'' |
        '*'...'[' |
        ']'...'~' => true,
        // obs-ctext
        _ => match tp {
            Ascii => false,
            Internationalized => ch.len_utf8() > 1
        }
    }
}

/// check if a char is a especial (based on RFC 5322)
pub fn is_special(ch: char ) -> bool {
    match ch {
        '(' | ')' |
        '<' | '>' |
        '[' | ']' |
        ':' | ';' |
        '@' | '\\'|
        ',' | '.' |
        '"' => true,
        _ => false
    }
}

/// check if a char is an tspecial (based on RFC 2045)
pub fn tspecial( ch: char ) -> bool {
    match char {
        '(' | ')' |
        '<' | '>' |
        '@' | ',' |
        ';' | ':' |
        '\\'| '"' |
        '/' | '[' |
        ']' | '?' |
        '=' => true,
        _ => false
    }
}



/// atext as defined by RFC 5322
#[inline(always)]
pub fn is_atext( ch: char, tp: MailType  ) -> bool {
    ( ! is_special( ch ) ) || {
        match tp {
            Ascii => false,
            Internationalized => ch.len_utf8() > 1
        }
    }
}

//qtext as defined by RFC 5322
pub fn is_qtext( ch: char, tp: MailType ) -> bool {
    match ch {
        //not ' ' [d:32]
        '!' |
        //not '"' [d:34]
        '#'...'[' |
        //not '\\' [d:92]
        ']'...'~' => true,
        //obs-qtext
        _ => match tp {
            Ascii => false,
            Internationalized => ch.len_utf8() > 1
        }
    }
}

/// is it a CTL (based on RFC 822)
#[inline(always)]
pub fn is_ctl( ch: char ) -> bool {
    (ch as u32) < 32
}


#[inline(always)]
pub fn is_especial( ch: char ) -> bool {
    match ch {
        '(' | ')' |
        '<' | '>' |
        '@' | ',' |
        ';' | ':' |
        '"' | '/'|
        '[' | ']' |
        '?' | '.' |
        '=' => true,
        _ => false
    }
}


//TODO thisshould be some where else I think
// (but it is used by `1. codec`, `2. components` )
/// based on RFC 2047
pub mod encoded_word {
    use nom;

    use error::*;

    use super::{ is_vchar, is_especial, is_ascii_vchar, is_ctl, MailType};

    #[derive(Debug, Copy, Clone, Hash, Eq, PartialEq)]
    pub enum EncodedWordContext {
        Phrase,
        Text,
        Comment
    }

    impl EncodedWordContext {

        fn char_validator( &self ) -> fn(char) -> bool {
            use self::EncodedWordContext::*;
            match *self {
                Phrase => valid_char_in_ec_in_phrase,
                Text => is_encoded_word_char,
                Comment => valid_char_in_ec_in_comment,
            }
        }
    }


    pub fn is_encoded_word( word: &str, ctx: EncodedWordContext ) -> bool {
        try_parse_encoded_word_parts( word, ctx ).is_ok()
    }

    pub fn try_parse_encoded_word_parts( word: &str, ctx: EncodedWordContext )
                                         -> Result<(&str, &str, &str)>
    {
        let char_validator = ctx.char_validator();
        // Note we could get a possible speed up by making rustc generate
        // a different function for each Context, inlining ALL char tests
        let res = do_parse!(
            word,
            char!( '=' ) >>
            char!( '?' ) >>
            charset: take_while!( is_ew_token_char ) >>
            char!( '?' ) >>
            encoding: take_while!( is_ew_token_char ) >>
            char!( '?' ) >>
            text: take_while!( char_validator ) >>
            eof!() >>
            (charset, encoding, text)
        );

        match res {
            nom::IResult::Done( rest, result ) => {
                //we used eof, so this should be true
                assert!( rest.len() == 0 );
                Some( result )
            },
            nom::IResult::Incomplete( .. ) => bail!( "incomplete encoded word: {}", word ),
            nom::IResult::Error( e ) => bail!( "malformed encoded word: {}", word )
        }
    }

    fn is_encoded_word_char( ch: char ) -> bool {
        is_ascii_vchar( ch ) && ch != '?'
    }


    fn valid_char_in_ec_in_comment( ch: char ) -> bool {
        is_encoded_word_char( ch ) && !( ch == '(' || ch == ')' || ch == '"' )
    }

    fn is_ew_token_char( ch: char ) -> bool {
        is_ascii_vchar( ch ) && !is_especial( ch )
    }




    fn valid_char_in_ec_in_phrase( ch: char ) -> bool {
        match ch {
            '0'...'9' |
            'a'...'z' |
            'A'...'Z' |
            '!' | '*' |
            '+' | '-' |
            '/' | '=' |
            '_' => true,
            _ => false
        }
    }

}



