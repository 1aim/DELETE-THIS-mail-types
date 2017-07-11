use std::ops::Range;

use ascii::AsciiChar;

use error::*;
use codec::{ MailEncoder, MailEncodable };
use super::shared::Item;
use super::components::data_types::Phrase;
use super::components::behaviour::encode::EncodeComponent;

#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct PhraseList {
    inner: Item,
    component_slices: Vec<Phrase>
}

impl MailEncodable for PhraseList {
    fn encode( &self, encoder: &mut MailEncoder ) -> Result<()> {
        if self.component_slices.len() == 0 {
            return Err( ErrorKind::AtLastOneElementIsRequired.into() );
        }
        sep_for!{ phrase in self.component_slices.iter();
            sep { encoder.write_char( AsciiChar::Comma ) };

            phrase.encode( &self.inner, encoder )?;
        }

        Ok( () )
    }
}

