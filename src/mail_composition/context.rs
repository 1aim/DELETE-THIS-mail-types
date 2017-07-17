
//TODO replace with types::ContentId
#[derive(Debug, Clone, Hash, Eq, PartialEq, Serialize)]
pub struct ContentId( String );

#[derive(Debug)]
pub struct Context {
    support: Support
}

impl Context {

    fn new_content_id( &self ) -> ContentId {

    }

    fn support( &self ) -> Support {
        self.support
    }
}

#[derive(Debug, Clone, Copy, Hash, Eq, PartialEq)]
pub enum Support {
    Ascii,
    //ascii but allows 8bit encodings for the body
    Ascii8BitMime,
    // requires smtputf8
    Internationalized
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
