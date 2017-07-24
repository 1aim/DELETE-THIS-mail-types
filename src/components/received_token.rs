use codec::{ MailEncoder, MailEncodable };

use error::*;
use super::shared::Item;
use super::components::data_types::{ Email, Domain, Word };
use super::components::behaviour::encode::EncodeComponent;

#[derive(Debug, Clone, Hash, Eq, PartialEq)]
enum Variant {
    Word( Word ),
    Address( Email ),
    Domain( Domain )
}

#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct ReceivedToken {
    inner: Item,
    component_slices: Variant
}

impl MailEncodable for ReceivedToken {
    fn encode( &self, encoder: &mut MailEncoder ) -> Result<()> {
        use self::Variant::*;
        match self.component_slices {
            Word( ref word ) => word.encode( &self.inner, encoder ),
            Address( ref addr ) => addr.encode( &self.inner, encoder ),
            Domain( ref domain ) => domain.encode( &self.inner, encoder )
        }
    }
}

