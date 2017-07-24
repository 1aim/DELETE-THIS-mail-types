use ascii::AsciiChar;

use error::*;
use codec::{ MailEncoder, MailEncodable };
use super::shared::Item;
use super::components::data_types::Email;
use super::components::behaviour::encode::EncodeComponent;

#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct Path {
    inner: Item,
    component_slices: Option<Email>
}


impl MailEncodable for Path {
    fn encode( &self, encoder: &mut MailEncoder ) -> Result<()> {
        encoder.note_optional_fws();
        encoder.write_char( AsciiChar::LessThan );
        if let &Some( ref email ) = &self.component_slices {
            email.encode( &self.inner, encoder )?;
        }
        encoder.write_char( AsciiChar::GreaterThan );
        encoder.note_optional_fws();
        Ok( () )
    }
}
//TODO for parsing we have to make sure to _require_ '<>' around the email