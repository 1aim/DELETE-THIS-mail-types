use ascii::{ AsciiString, AsAsciiStr };
use codec::MailEncoder;
use quoted_printable::encode;

pub fn q_encode( input: &str , encoder: &mut MailEncoder ) {
    //TODO I suspect the `quoted_printable` crate is not
    // completely correct wrt. to some aspects, have to
    // check this
    //FIXME does need the current line length and wather or not it is a header
    let raw = encode( input.as_bytes() );
    let asciied = unsafe { AsciiString::from_ascii_unchecked( raw ) };
    encoder.write_str( &*asciied )
}

pub fn puny_code_domain(_input: &str, _encoder: &mut MailEncoder ) {
    if let Ok( val ) = _input.as_ascii_str() {
        _encoder.write_str( val )
    } else {
        unimplemented!();
    }
}