//! This module provides utilities for composing multipart mails.
//!
//! While the `Mail` type on itself can represent any multipart
//! mail most mails have a certain pattern to their structure,
//! consisting mainly of `multipart/mixed` for attachments,
//! `multipart/alternative` for alternative bodies and
//! `multipart/related` for including embedded resources which
//! can be used in the mail bodies  like e.g. a logo.
//!
//! This module provides the needed utilities to more simply
//! create a `Mail` instance which represents this kind of
//! mails.

//-------------------------------------------------------------\\
// NOTE: Implementations for creating (composing) mails are    ||
// split from the type dev, and normal impl blocks and placed  ||
// in the later part of the file for better readability.       ||
//-------------------------------------------------------------//

use media_type::{MULTIPART, ALTERNATIVE, RELATED, MIXED};
use vec1::Vec1;

#[cfg(feature="serde")]
use serde::{Serialize, Deserialize};

use headers::{
    HeaderKind,
    headers,
    header_components::{
        ContentId,
        Disposition,
        DispositionKind,
        MediaType
    }
};

use ::mail::Mail;
use ::context::Context;
use ::resource::Resource;


/// Parts used to create a mail body (in a multipart mail).
///
/// This type contains a `Resource` which is normally used
/// to create a alternative body in a `multipart/alternative`
/// section. As well as a number of "embeddings" which depending
/// on there disposition can are either used as attachments
/// or embedded resource to which the body can referre to
/// by it's content id.
#[derive(Debug)]
pub struct BodyPart {

    /// A body created by a template.
    pub resource: Resource,

    //TODO split in inline_embeddings, attachments ->
    /// Embeddings added by the template engine.
    ///
    /// It is a mapping of the name under which a embedding had been made available in the
    /// template engine to the embedding (which has to contain a CId, as it already
    /// was used in the template engine and CIds are used to link to the content which should
    /// be embedded)
    pub embeddings: Vec<Embedded>,

}

/// Parts which can be used to compose a multipart mail.
///
/// This can be used to crate a mail, possible having
/// attachments with multiple alternative bodies having
/// embedded resources which can be referred to by the
/// bodies with content ids. This embeddings can be both
/// body specific or shared between bodies.
///
/// # Limitations
///
/// Any non alternative body will be either an attachment
/// or an body with a inline disposition header in a
/// `multipart/related` body. Which means you can not
/// use this mechanism to e.g. create a `multipart/mixed`
/// body with multiple disposition inline sub-bodies
/// which should be displayed side-by-side. Generally this
/// is not a very good way to create a mail, through a
/// valid way nevertheless.
///
pub struct MailParts {
    /// A vector of alternative bodies
    ///
    /// A typical setup would be to have two alternative bodies one text/html and
    /// another text/plain as fallback (for which the text/plain body would be
    /// the first in the vec and the text/html body the last one).
    ///
    /// Note that the order in the vector     /// a additional text/plainis
    /// the same as the order in which they will appear in the mail. I.e.
    /// the first one is the last fallback while the last one should be
    /// shown if possible.
    pub alternative_bodies: Vec1<BodyPart>,

    //TODO split in to vec inline_embeddings, attachments
    /// A number of embeddings.
    ///
    /// Depending on the disposition of the embeddings they will be either
    /// used as attachments or as embedded resources to which bodies can
    /// refer by there content id. In difference to the `embeddings` field
    /// in `BodyParts` embedded resources placed here can be used in all
    /// bodies created by `alternative_bodies`.
    pub embeddings: Vec<Embedded>
}


/// A resource embedded in a mail.
///
/// Depending on the deposition this will either be used
/// to create a attachment or a embedded resources other
/// resources can refer to by the resources content id.
///
#[derive(Debug, Clone)]
#[cfg_attr(feature="serde", derive(Serialize, Deserialize))]
pub struct Embedded {
    content_id: Option<ContentId>,
    resource: Resource,
    disposition: DispositionKind,
}

impl Embedded {

    /// Create a inline embedding from an `Resource`.
    pub fn inline(resource: Resource) -> Self {
        Embedded::new(resource, DispositionKind::Inline)
    }

    /// Create a attachment embedding from an `Resource`.
    pub fn attachment(resource: Resource) -> Self {
        Embedded::new(resource, DispositionKind::Attachment)
    }

    /// Create a new embedding from a resource using given disposition.
    pub fn new(resource: Resource, disposition: DispositionKind) -> Self {
        Embedded {
            content_id: None,
            resource,
            disposition
        }
    }

    /// Create a new embedding from a `Resource` using given disposition and given content id.
    pub fn with_content_id(resource: Resource, disposition: DispositionKind, content_id: ContentId) -> Self {
        Embedded {
            content_id: Some(content_id),
            resource,
            disposition
        }
    }

    /// Return a reference to the contained resource.
    pub fn resource(&self) -> &Resource {
        &self.resource
    }

    /// Return a mutable reference to the contained resource.
    pub fn resource_mut(&mut self) -> &mut Resource {
        &mut self.resource
    }

    /// Return a reference to the contained content id, if any.
    pub fn content_id(&self) -> Option<&ContentId> {
        self.content_id.as_ref()
    }

    /// Return a reference to disposition to use for the embedding.
    pub fn disposition(&self) -> DispositionKind {
        self.disposition
    }

    /// Generate and set a new content id if this embedding doesn't have a content id.
    pub fn assure_content_id(&mut self, ctx: &impl Context) -> &ContentId {
        if self.content_id.is_none() {
            self.content_id = Some(ctx.generate_content_id());
        }

        self.content_id().unwrap()
    }
}


//-------------------------------------------------------\\
//  implementations for creating mails are from here on  ||
//-------------------------------------------------------//


impl MailParts {

    /// Generating content ids for all contained `Embedded` instances which don't have a cid.
    ///
    pub fn generate_content_ids(&mut self, ctx: &impl Context) {
        for body in self.alternative_bodies.iter_mut() {
            for embedding in body.embeddings.iter_mut() {
                embedding.assure_content_id(ctx);
            }
        }

        for embedding in self.embeddings.iter_mut() {
            embedding.assure_content_id(ctx);
        }
    }


    /// Create a `Mail` instance based on this `MailParts` instance.
    ///
    /// This will first generate content ids for all contained
    /// `Embedded` instances.
    ///
    /// If this instance contains any attachments then the
    /// returned mail will be a `multipart/mixed` mail with
    /// the first body containing the actual mail and the
    /// other bodies containing the attachments.
    ///
    /// If the `MailParts.embeddins` is not empty then the
    /// mail will be wrapped in `multipart/related` (inside
    /// any potential `multipart/mixed`) containing hte
    /// actual mail in the first body and the embeddings
    /// in the other bodies.
    ///
    /// The mail will have a `multipart/alternative` body
    /// if it has more then one alternative body
    /// (inside a potential `multipart/related` inside a
    /// potential `multipart/mixed` body). This body contains
    /// one sub-body for each `BodyPart` instance in
    /// `MailParts.alternative_bodies`.
    ///
    /// Each sub-body created for a `BodyPart` will be wrapped
    /// inside a `multipart/related` if it has body specific
    /// embeddings (with content disposition inline).
    pub fn compose_mail(mut self, ctx: &impl Context)
        -> Mail
    {
        self.generate_content_ids(ctx);
        self.compose_without_generating_content_ids()
    }

    /// This function works like `compose_mail` but does not generate
    /// any content ids.
    pub fn compose_without_generating_content_ids(self)
        -> Mail
    {
        let MailParts { alternative_bodies, embeddings } = self;

        let mut attachments = Vec::new();
        let mut alternatives = alternative_bodies.into_iter()
            .map(|body| body.create_mail(&mut attachments))
            .collect::<Vec<_>>();

        let embeddings = embeddings.into_iter()
            .filter_map(|emb| {
                let disposition = emb.disposition();
                let mail = emb.create_mail();
                if disposition == DispositionKind::Attachment {
                    attachments.push(mail);
                    None
                } else {
                    Some(mail)
                }
            })
            .collect::<Vec<_>>();

        //UNWRAP_SAFE: bodies is Vec1, i.e. we have at last one
        let mail = alternatives.pop().unwrap();
        let mail =
            if alternatives.is_empty() {
                mail
            } else {
                mail.wrap_with_alternatives(alternatives)
            };

        let mail =
            if embeddings.is_empty() {
                mail
            } else {
                mail.wrap_with_related(embeddings)
            };

        let mail =
            if attachments.is_empty() {
                mail
            } else {
                mail.wrap_with_mixed(attachments)
            };

        mail
    }
}

impl BodyPart {

    /// Creates a `Mail` instance from this `BodyPart` instance.
    ///
    /// All embeddings in `BodyPart.embeddings` which have a
    /// attachment content disposition are placed into the
    /// `attachments_out` parameter, as attachments should
    /// always be handled on the outer most level but the
    /// produced mail is likely not the outer most level.
    ///
    /// This will create a non-multipart body for the
    /// body `Resource`, if there are any embeddings which
    /// have a `Inline` disposition that body will be
    /// wrapped into a `multipart/related` body containing
    /// them.
    pub fn create_mail(self, attachments_out: &mut Vec<Mail>)
        -> Mail
    {
        let BodyPart { resource, embeddings } = self;
        let body = resource.create_mail();

        let related = embeddings.into_iter()
            .filter_map(|embedded| {
                let disposition = embedded.disposition();
                let emb_mail = embedded.create_mail();
                if disposition == DispositionKind::Attachment {
                    attachments_out.push(emb_mail);
                    None
                } else {
                    Some(emb_mail)
                }
            })
            .collect::<Vec<_>>();

        if related.is_empty() {
            body
        } else {
            body.wrap_with_related(related)
        }
    }
}

impl Embedded {

    /// Create a `Mail` instance for this `Embedded` instance.
    ///
    /// This will create a non-multipart mail based on the contained
    /// resource containing a `Content-Disposition` header as well as an
    /// `Content-Id` header if it has a content id.
    ///
    pub fn create_mail(self) -> Mail {
        let Embedded {
            content_id,
            resource,
            disposition:disposition_kind
        } = self;

        let mut mail = resource.create_mail();
        if let Some(content_id) = content_id {
            mail.insert_header(headers::ContentId::body(content_id));
        }
        let disposition = Disposition::new(disposition_kind, Default::default());
        mail.insert_header(headers::ContentDisposition::body(disposition));
        mail
    }
}

impl Resource {

    /// Create a `Mail` instance representing this `Resource`.
    ///
    /// This is not a complete mail, i.e. it will not contain
    /// headers like `From` or `To` and in many cases the
    /// returned `Mail` instance will be wrapped into other
    /// mail instances adding alternative bodies, embedded
    /// resources and attachments.
    pub fn create_mail(self) -> Mail {
        Mail::new_singlepart_mail(self)
    }
}

impl Mail {

    /// Create a `multipart/mixed` `Mail` instance containing this mail as
    /// first body and one additional body for each attachment.
    ///
    /// Normally this is used with embeddings having a attachment
    /// disposition creating a mail with attachments.
    pub fn wrap_with_mixed(self, other_bodies: Vec<Mail>)
        -> Mail
    {
        let mut bodies = other_bodies;
        bodies.push(self);
        new_multipart(&MIXED, bodies)
    }

    /// Create a `multipart/alternative` `Mail` instance containing this
    /// mail as the _main_ body with given alternatives.
    ///
    /// The "priority" of alternative bodies is ascending with the body
    /// which should be shown only if all other bodies can't be displayed
    /// first. I.e. the order is the same order as
    /// specified by `multipart/alternative`.
    /// This also means that _this_ body will be the last body as it is
    /// meant to be the _main_ body.
    pub fn wrap_with_alternatives(self, alternates: Vec<Mail>)
        -> Mail
    {
        let mut bodies = alternates;
        bodies.insert(0, self);
        new_multipart(&ALTERNATIVE, bodies)
    }

    /// Creates a `multipart/related` `Mail` instance containing this
    /// mail first and then all related bodies.
    pub fn wrap_with_related(self, related: Vec<Mail>)
        -> Mail
    {
        let mut bodies = related;
        bodies.insert(0, self);
        new_multipart(&RELATED, bodies)
    }

}

/// Creates a `multipart/<sub_type>` mail with given bodies.
///
/// # Panic
///
/// If `sub_type` can not be used to create a multipart content
/// type this will panic.
fn new_multipart(sub_type: &'static str, bodies: Vec<Mail>)
    -> Mail
{
    let content_type = MediaType::new(MULTIPART, sub_type)
        .unwrap();
    Mail::new_multipart_mail(content_type, bodies)
}