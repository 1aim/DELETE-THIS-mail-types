use ascii::AsciiChar;

use error::*;
use codec::{ SmtpDataEncoder, SmtpDataEncodable };
use types::shared::Item;
use types::components::data_types::Email;
use types::components::behaviour::encode::EncodeComponent;

pub struct MessageID {
    inner: Item,
    //FIXME there are actullay some differences (mainly (only?) no folding allowed)
    component_slices: Email
}

impl SmtpDataEncodable for MessageID {
    fn encode( &self, encoder: &mut SmtpDataEncoder ) -> Result<()> {
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

impl SmtpDataEncodable for MessageIDList {
    fn encode( &self, encoder: &mut SmtpDataEncoder ) -> Result<()> {
        for msg_id in self.0.iter() {
            msg_id.encode( encoder )
        }
    }
}