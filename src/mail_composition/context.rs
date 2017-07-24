use std::sync::Arc;
use mail::BuilderContext;

//TODO replace with types::ContentId
#[derive(Debug, Clone, Hash, Eq, PartialEq, Serialize)]
pub struct ContentId( String );


trait Context: BuilderContext {
    fn new_content_id( &self ) -> ContentId;
}

impl<T: Context> Context for Arc<T> {
    fn new_content_id( &self ) -> ContentId {
        (*self).new_content_id()
    }
}



type Mailbox = TODO:

pub struct MailSendContext {
    pub from: Mailbox,
    pub to: To,
    pub subject: String
}



pub enum To {
    Mailbox( Mailbox ),
    Email( Email )
}

impl To {
    fn display_name_or_else<F>(self, func: F) -> Self
        where F: FnOnce() -> Option<String>
    {
        match self {
            To::Mailbox( mbox ) => To::Mailbox( mbox ),
            To::Email( email ) => {
                let display_name = func();
                To::Mailbox( Mailbox::from_email( display_name, email ) )
            }
        }
    }
}
