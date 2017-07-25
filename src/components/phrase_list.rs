use std::ops::Range;

use ascii::AsciiChar;

use error::*;
use codec::{MailEncoderImpl, MailEncodable };

use super::Phrase;


#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct PhraseList(pub Vec1<Phrase>);


impl MailEncodable for PhraseList {
    fn encode<E>( &self, encoder:  &mut E ) -> Result<()>
        where E: MailEncoder
    {
        sep_for!{ phrase in self.0.iter();
            sep { encoder.write_char( AsciiChar::Comma ) };

            phrase.encode( &self.inner, encoder )?;
        }

        Ok( () )
    }
}

