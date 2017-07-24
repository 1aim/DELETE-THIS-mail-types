use std::ops::Range;

use ascii::AsAsciiStr;

use error::*;
use codec::{ MailEncoder, MailEncodable };
use super::shared::Item;
use super::components::data_types::{View, Email};
use super::components::behaviour::encode::EncodeComponent;
use char_validators::encoded_word::EncodedWordContext;

#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct Text {
    inner: Item,
    //TODO maybe create a text component
    component_slices: Range<usize>
}


impl MailEncodable for Text {
    fn encode( &self, encoder: &mut MailEncoder ) -> Result<()> {
        let text = self.component_slices.apply_on( &self.inner );
        if let Ok( as_ascii ) = text.as_ascii_str() {
            encoder.write_str( as_ascii );
        } else {
            //TODO auto splitting into multiple encoded words (length is limited to 75)
            //Text(here) corresponds to *text with text being a single character in the rfc
            //as such we can split it at any point, not that we still cant put line breakes
            //in there **encoded words in have to parsable as a single token** do not confuse
            //with qutable-encoding on itself
            encoder.write_encoded_word( text, EncodedWordContext::Text )
        }
        Ok( () )
    }
}