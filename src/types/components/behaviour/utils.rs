
///WS as defined by RFC 5234
#[inline(always)]
pub fn is_ws(ch: char) -> bool {
    // is not limited to ascii ws
    //ch.is_whitespace()
    //WSP            =  SP / HTAB
    ch == ' ' || ch == '\t'
}

//VCHAR as defined by RFC 5243
pub fn is_vchar(ch: char) -> bool {
    unimplemented!();
    //%x21-7E            ; visible (printing) characters
}

///any whitespace (char::is_whitespace
#[inline(always)]
pub fn is_any_whitespace(ch: char) -> bool {
    ch.is_whitespace()
}

//ctext as defined by RFC 5322
pub fn is_ctext(ch: char) -> bool {
    //%d33-39 /          ; Printable US-ASCII
    //%d42-91 /          ;  characters not including
    //%d93-126 /         ;  "(", ")", or "\"
    // obs-ctext
    unimplemented!();
}


//atext as defined by RFC 5322
pub fn is_atext(ch: char) -> bool {
    unimplemented!();
    // ALPHA / DIGIT /   ; Printable US-ASCII
    //"!" / "#" /        ;  characters not including
    //"$" / "%" /        ;  specials.  Used for atoms.
    //"&" / "'" /
    //"*" / "+" /
    //"-" / "/" /
    //"=" / "?" /
    //"^" / "_" /
    //"`" / "{" /
    //"|" / "}" /
    //"~"
}

//qtext as defined by RFC 5322
pub fn is_qtext(ch: char) -> bool {
    unimplemented!();
    //%d33 /             ; Printable US-ASCII
    //%d35-91 /          ;  characters not including
    //%d93-126 /         ;  "\" or the quote character
    // obs-qtext
}
