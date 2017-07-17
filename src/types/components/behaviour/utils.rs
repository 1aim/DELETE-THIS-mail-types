use self::MailType::*;

pub enum MailType {
    Ascii,
    Internationalized
    //TODO add include/exclude obsolete
}

///WS as defined by RFC 5234
#[inline(always)]
pub fn is_ws(ch: char) -> bool {
    // is not limited to ascii ws
    //ch.is_whitespace()
    //WSP            =  SP / HTAB
    ch == ' ' || ch == '\t'
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


pub fn is_special( ch: char ) -> bool {
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
