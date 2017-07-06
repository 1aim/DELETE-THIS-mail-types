use error::*;
use super::shared::Item;
use super::components::data_types;
use super::components::behaviour::encode::EncodeComponent;
use codec::{ SmtpDataEncoder, SmtpDataEncodable };

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Unstructured {
    inner: Item,
    //FIXME check if needed
    component_slices: data_types::Unstructured
}

impl SmtpDataEncodable for Unstructured {
    fn encode( &self, encoder: &mut SmtpDataEncoder ) -> Result<()> {
        self.component_slices.encode( &self.inner, encoder )
    }
}
