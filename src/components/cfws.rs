use error::*;
use codec::{MailEncoderImpl, MailEncodable };

//FEATURE_TODO(fws_controll): allow controlling the amount of WS and if a CRLF should be used in FWS
//  this is also usefull for parsing and keeping information about FWS structure
//FEATURE_TODO(cfws_with_comments): allow encoding commnets in CFWS
//  this allows encoding comments in CFWS, combine with (part of?) fws_controll
//  required (partially) for parsing comments (through skipping them works without this)

//
//pub enum WS {
//    TAB,
//    SPACE
//}
//
//pub struct FWS(pub Option<WithCRLF>, pub Vec1<WS> );
//
//pub struct WithCRLF {
//    pub trailing: Vec<WS>
//}

pub enum CFWS {
    //WithComment( Vec1<(Option<FWS>, Comment)>, Option<FWS> ),
    SingleFws( FWS )
}


impl MailEncodable for CFWS {

    fn encode<E>( &self, encoder:  &mut E ) -> Result<()>
        where E: MailEncoderImpl
    {
        match self {
            CFWS::SingleFws(ref FWS ) => {
                encoder.write_fws()
            }
        }
    }

}