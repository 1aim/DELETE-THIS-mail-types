use error::*;
use super::shared::Item;
use super::components::data_types;
use super::components::behaviour::encode::EncodeComponent;
use codec::{ MailEncoder, MailEncodable };

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Unstructured {
    inner: Item,
    //FIXME check if needed
    component_slices: data_types::Unstructured
}

impl MailEncodable for Unstructured {
    fn encode( &self, encoder: &mut MailEncoder ) -> Result<()> {
        self.component_slices.encode( &self.inner, encoder )
    }
}
