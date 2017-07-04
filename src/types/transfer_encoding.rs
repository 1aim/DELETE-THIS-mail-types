use ascii::AsciiString;
//TODO

pub enum TransferEncoding {
    _7Bit,
    _8Bit,
    Binary,
    QuotedPrintable,
    Base64,
    XToken( Token ),
    // should be only ietf-token (i.e. tokens standarized through an RFC and registered with IANA)
    // but we don't check this so it's other and not ietf token
    //FIXME not sure if the limitations are to tight (with Token)
    //FIXME allow puting XTokens into OtherToken when generating?
    OtherToken( Token ),
}


//FIXME limit chars valid for token (no space, no special chars like {([" ... )
pub struct Token( AsciiString );