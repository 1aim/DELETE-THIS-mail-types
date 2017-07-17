use std::collections::BTreeMap;
use std::mem::replace;

use super::resource;
use super::resource::Resource;
use super::context::{ Context, ContentId };

pub trait SearchData {

    fn find_externals<F1,F2>( &mut self, emb: F1, att: F2 )
        where F1: FnMut( &mut Embedding ),
              F2: FnMut( &mut Attachment);
}

//FIXME PathBuf => FileSource
#[derive(Debug)]
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

//FIXME PathBuf => FileSource
#[derive(Debug)]
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


pub fn preprocess_data<D: SearchData>( ctx: &Context, data: &mut D )
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
