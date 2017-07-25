use error::*;
use super::shared::Item;
use super::components::data_types;
use super::components::behaviour::encode::EncodeComponent;
use codec::{MailEncoderImpl, MailEncodable };

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Unstructured {
    inner: Item,
    //FIXME check if needed
    component_slices: data_types::Unstructured
}

impl MailEncodable for Unstructured {
    fn encode<E>( &self, encoder:  &mut E ) -> Result<()> where E: MailEncoder {
        //FIXME can contain encoded-word
        //FIXME replace usage of Text with Unstructured at all points
        self.component_slices.encode( &self.inner, encoder )
    }
}
