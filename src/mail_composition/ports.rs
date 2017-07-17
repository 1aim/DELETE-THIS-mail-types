use mime::Mime;
use serde::Serialize;

use error::*;

pub use super::data::{
    Attachment as AttachmentIn,
    Embedding as EmbeddingIn,
    SearchData
};
pub use super::resource::{
    Embedding as EmbeddingOut,
    Attachment as AttachmentOut,
    Resource,
    Stream
};

pub trait TemplateEngine {
    type TemplateId;

    //FIXME use Vec1Plus or something
    fn templates<D: Serialize>
        ( ctx: &Context, id: Self::TemplateId, data: D )
        -> Result< Vec<Template> >;

    //FIXME move into custom Trait
    fn compose_display_name( ctx: &Context, data: &Self::Data ) -> Option<String>;
}

pub struct Template {
    pub mime: Mime,
    pub data: Stream,
    pub embeddings: Vec< EmbeddingOut >,
    pub attachments: Vec< AttachmentOut >
}
