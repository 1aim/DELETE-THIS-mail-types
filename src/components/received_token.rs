use ascii::AsciiChar;

use error::*;
use codec::{ MailEncoder, MailEncodable };
use super::word::{ Word, do_encode_word };
use super::{ Email, Domain };


#[derive( Debug, Clone, Eq, PartialEq, Hash )]
pub struct ReceivedTokenWord( Word );

impl ReceivedTokenWord {
    pub fn new( item: InnerAsciiItem ) -> Result<Self> {
        Ok( PhraseWord( Word::new( item, true )? ) )
    }

    pub fn from_parts(
        left_padding: Option<CFWS>,
        item: InnerAsciiItem,
        right_padding: Option<CFWS>,
    ) -> Result<Self> {
        Ok( PhraseWord( Word::from_parts( left_padding, item, right_padding, true )? ) )
    }

}

deref0!{ +mut ReceivedTokenWord, Word }

#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub enum ReceivedToken {
    Word( ReceivedTokenWord ),
    Address( Email ),
    Domain( Domain )
}

impl MailEncodable for ReceivedToken {
    fn encode<E>( &self, encoder:  &mut E ) -> Result<()> where E: MailEncoder {
        use self::Variant::*;
        match self.component_slices {
            Word( ref word ) => {
                do_encode_word( word, encoder, None )?;
            },
            Address( ref addr ) => {
                // we do not need to use <..> , but I think it's better and it is definitely
                // not wrong
                encoder.write_char( AsciiChar::LessThan );
                addr.encode( encoder )?;
                encoder.write_char( AsciiChar::GreaterThan );
            },
            Domain( ref domain ) => {
                domain.encode( encoder )?;
            }
        }
        Ok( () )
    }
}

