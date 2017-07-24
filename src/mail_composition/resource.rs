use std::fmt;

use mime;
use futures::future::BoxFuture;
use ascii::AsciiChar;

use error::*;
use utils::Buffer;
use raw_mail::resource::Resource;

pub type Embeddings = Vec<EmbeddingInMail>;
pub type Attachments = Vec<AttachmentInMail>;

pub type AttachmentInMail = Resource;

#[derive(Debug)]
pub struct EmbeddingInMail {
    pub content_id: ContentId,
    pub resource: Resource
}
