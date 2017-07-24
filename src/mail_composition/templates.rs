use serde::Serialize;

use error::*;
use types::Vec1;
use mail::resource::Resource;

use super::context::Context;
use super::resource::{
    EmbeddingInMail,
    AttachmentInMail,
    Embeddings,
    Attachments
};

pub trait TemplateEngine {
    type TemplateId;

    fn templates<D: Serialize, C: Context>( ctx: &C, id: Self::TemplateId, data: D )
                                -> Result< Vec1<Template> >;
}


pub struct Template {
    pub body: Resource,
    pub embeddings: Embeddings,
    pub attachments: Attachments
}