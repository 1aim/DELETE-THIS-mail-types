use ascii::AsciiChar;

use error::*;
use codec::{ MailEncoder, MailEncodable };
use components::shared::Item;
use components::components::data_types::Email;
use components::components::behaviour::encode::EncodeComponent;

pub struct MessageID {
    inner: Item,
    //FIXME there are actullay some differences (mainly (only?) no folding allowed)
    component_slices: Email
}

impl MailEncodable for MessageID {
    fn encode( &self, encoder: &mut MailEncoder ) -> Result<()> {
        encoder.write_char( AsciiChar::LessThan );
        self.component_slices.encode( &self.inner, encoder )?;
        encoder.write_char( AsciiChar::GreaterThan );
        Ok( () )
    }
}


pub struct MessageIDList( Vec<MessageID> );

impl MessageIDList {

    pub fn new_with_first( first: MessageID ) -> Self {
        MessageIDList( vec![ first ] )
    }

    pub fn new( list: Vec<MessageID> ) -> Result<Self> {
        if list.is_empty() {
            Err( ErrorKind::AtLastOneElementIsRequired.into() )
        } else {
            Ok( MessageIDList( list ) )
        }
    }

    pub fn push( &mut self, id: MessageID ) {
        self.0.push( id )
    }

}

impl MailEncodable for MessageIDList {
    fn encode( &self, encoder: &mut MailEncoder ) -> Result<()> {
        for msg_id in self.0.iter() {
            msg_id.encode( encoder );
        }
        Ok( () )
    }
}

//TODO for parsing we have to make sure to _require_ '<>' around the email