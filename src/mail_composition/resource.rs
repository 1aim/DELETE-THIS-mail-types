use

pub type Attachment = Resource;

pub enum Stream {
    Ascii( BoxStream<Item=AsciiChar, Error=Error> ),
    NonAscii( BoxStream<Item=AsciiChar, Error=Error> )
}

impl Stream {

    fn is_ascii( &self ) -> bool {
        use self::Stream::*;
        match *self {
            Ascii( .. ) => true,
            NonAscii( .. ) => false
        }
    }

}


pub enum Resource {
    // mime - name - source
    // TODO make Resource a struct and only the last fied a either variant
    Inline( mime::Mime, Option<String>, Stream ),
    File( mime::Mime, Option<String>, PathBuf )
}

impl fmt::Debug for Resource {

    fn fmt( &self, fter: &mut fmt::Formatter ) -> fmt::Result {
        use self::Resource::*;
        write!{ fter, "Resource::" }?;
        match *self {
            Inline( ref mime, ref name, .. ) => {
                write!( fter, "Inline( {}, {:?}, Steam{{..}} )", mime, name )?;
            },
            File( ref mime, ref name, ref path ) => {
                write!( fter, "File( {}, {:?}, {:?} )", mime, name, path )?;
            }
        }
        Ok( () )
    }
}

#[derive(Debug)]
pub struct Embedding {
    id: ContentId,
    resource: Resource
}

impl Embedding {
    fn new(ctx: &Context, res: Resource ) -> Embedding {
        Embedding {
            id: ctx.new_content_id(),
            resource: res
        }
    }
}