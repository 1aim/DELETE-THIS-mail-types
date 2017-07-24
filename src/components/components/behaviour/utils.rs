use self::MailType::*;

//TODO move all is_... to a more general module

pub enum MailType {
    Ascii,
    Internationalized
    //TODO add include/exclude obsolete
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

//VCHAR as defined by RFC 5243
pub fn is_vchar( ch: char, tp: MailType ) -> bool {
    match ch {
        '!'...'~' => true,
        _ => match tp {
            Ascii => false,
            Internationalized => ch.len_utf8() > 1
        }
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

/// check if a char is a tspecial (based on RFC 2045)
pub fn is_tspecial(ch: char ) -> bool {
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

/// atext as defined by RFC 5322
#[inline(always)]
pub fn is_atext( ch: char, tp: MailType  ) -> bool {
    ( ! is_tspecial( ch ) ) || {
        match tp {
            Ascii => false,
            Internationalized => ch.len_utf8() > 1
        }
    }
}

//qtext as defined by RFC 5322
pub fn is_qtext( ch: char, tp: MailType ) -> bool {
    match ch {
        '!' |
        '#'...'[' |
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

//FIXME add internationalization extensiosn
/// is a char which can appear in a token (based on RFC 2045)
#[inline(always)]
pub fn is_token_char( ch: char ) -> bool {
    !is_space( ch ) && !is_ctl( ch ) && !is_tspecial( ch )
}
