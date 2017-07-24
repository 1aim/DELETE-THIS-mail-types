use super::super::data_types::*;

//TODO add smtputf8 support

//TODO potentaillay use custom error (quick_error ?)
use error::*;


trait DecodeComponent: Sized {
    // data will be the "full" data needed, as we will use a hirachical parser
    // 1. layer: *(<heder_name> : <some_content>) empty_line body
    //    without parsing some_content except checking if a newline is a FWS (CRLFWS)
    // 2. use ragnes from first layer to get heade neame -> determine body parser,
    //    get body data (including potential FWS) -> parse body
    // 3. [NEEDED??] decode encoded-words etc.
    /// parse component (i.e. slice), data is
    /// the complete slice, not more and nicer less bytes/characters
    /// through is might contains FWS sequences like <CR LF SPACE>
    fn parse( data: &[u8] ) -> Result<Self>;
}




impl DecodeComponent for Domain {

    //FIXME support domain-literal / obs-domain
    fn parse( _data: &[u8] ) -> Result<Self> {
        unimplemented!();
    }
}

#[cfg(excluded)]
mod parser;


