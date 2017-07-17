use std::collections::BTreeMap;
use std::mem::replace;

use serde;

use super::resource;
use super::resource::Resource;
use super::context::{ Context, ContentId };

type Mailbox = TODO;

pub trait DataInterface {

    fn find_externals<F1,F2>( &mut self, emb: F1, att: F2 )
        where F1: FnMut( &mut Embedding ),
              F2: FnMut( &mut Attachment);

    fn see_from_mailbox(&mut self, mbox: &Mailbox );
    fn see_to_mailbox(&mut self, mbox: &Mailbox );
}

//FIXME PathBuf => FileSource
#[derive(Debug, Serialize)]
pub struct Embedding(InnerEmbedding);
#[derive(Debug)]
enum InnerEmbedding {
    AsValue( Resource ),
    AsContentId( ContentId )
}

impl Embedding {
    pub fn new( resource: Resource ) -> Self {
        Embedding( InnerEmbedding::AsValue( resource ) )
    }

    fn swap_with_content_id( &mut self, cid: ContentId ) -> Option<Resource> {
        use self::InnerEmbedding::*;
        match replace( &mut self.0, AsContentId( ContentId ) ) {
            //TODO warn this is definitily a bug
            AsContentId( cid ) => None,
            AsValue( value ) => Some( value )
        }
    }
}

impl serde::Serialize for InnerEmbedding {
    fn serialize<S>( &self, serializer: S ) -> Result<S::Ok, S::Error>
        where S: Serializer
    {
        use self::InnerEmbedding::*;
        match self {
            AsValue( .. ) => Err( S::Error::custom( concat!(
                "embeddings can be serialized as content id, not as value, "
                "preprocess_data should have ben called before" ) ) ),
            AsContentId( cid ) => cid.serialize( serializer )
        }
    }
}

//FIXME PathBuf => FileSource
#[derive(Debug, Serialize)]
pub struct Attachment(InnerAttachment );
#[derive(Debug)]
enum InnerAttachment {
    AsValue( Resource ),
    /// the resource was moved out of data, to be added to the
    /// mail attachments
    Moved
}

impl Attachment {
    pub fn new( resource: Resource ) -> Self {
        Attachment( InnerAttachment::AsValue( resource ) )
    }

    fn move_out( &mut self ) -> Option<Resource> {
        use self::InnerAttachment::*;
        match replace( &mut self.0, InnerAttachment::Moved ) {
            AsValue( value ) => Some( value ),
            //TODO warn as this is likely a bug
            Moved => None
        }
    }
}

impl serde::Serialize for InnerAttachment {
    fn serialize<S>( &self, serializer: S ) -> Result<S::Ok, S::Error>
        where S: Serializer
    {
        use self::InnerAttachment::*;
        match self {
            AsValue( .. ) => Err( S::Error::custom( concat!(
                "only moved attachments can be serialized, "
                "preprocess_data should have ben called before" ) ) ),
            Moved => serializer.serialize_none()
        }
    }
}

pub fn preprocess_data<D: DataInterface>( ctx: &Context, data: &mut D )
    -> (Vec<resource::Embedding>, Vec<resource::Attachment>)
{
    let mut embeddings = Vec::new();
    let mut attachments = Vec::new();
    data.find_externals(
        |embedding| {
            if let Some( embedding ) = embedding.swap_with_content_id( ctx.new_content_id() ) {
                embeddings.push( embedding )
            }
        }
            |attachment| {
            if let Some( attachment ) = attachment.move_out() {
                attachments.push( attachment )
            }
        }
    )

        (embeddings, attachments)
}
