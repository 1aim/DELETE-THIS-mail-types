//! Module containing all the parts for creating/encoding Mails.
//!
//TODO[NOW]: work flow documentation

use std::ops::Deref;
use std::fmt;

use soft_ascii_string::SoftAsciiString;
use futures::{ future, Future, Async, Poll };

use common::MailType;
use common::encoder::{EncodableInHeader, EncodingBuffer};
use headers::{
    HeaderTryInto, Header, HeaderMap,
    ContentType, _From,
    ContentTransferEncoding,
    Date, MessageId
};
use headers::components::DateTime;
use headers::error::{
    HeaderValidationError, BuildInValidationError
};

use ::error::{MailError, BuilderError};
use ::context::Context;

use self::builder::{ check_header, check_multiple_headers };
pub use self::builder::{ Builder, MultipartBuilder, SinglepartBuilder };
pub use self::resource::*;

pub mod context;
mod resource;
mod builder;
mod encode;

/// A type representing a Mail.
///
/// This type is used to represent a mail including headers and body.
/// It is also used for the bodies of multipart mime mail bodies as
/// they can be seen as "sub-mails" or "hirachical nested mails", at
/// last wrt. everything relevant on this type.
///
/// A mail can be created using the `Builder` or more specific either
/// the `SinglepartBuilder` or the `MultipartBuilder` for a multipart
/// mime mail.
///
/// # Example
///
/// This will create, encode and print a simple plain text mail.
///
/// ```
/// # extern crate futures;
/// # extern crate mail_types;
/// # extern crate mail_common;
/// # #[macro_use] extern crate mail_headers as headers;
/// # use futures::Future;
/// # use mail_common::MailType;
/// # use headers::components::Domain;
/// use std::str;
/// // either from `mail::headers` or from `mail_header as headers`
/// use headers::*;
/// use mail_types::{
///     Mail, Resource,
///     default_impl::simple_context
/// };
///
/// # fn main() {
/// // Domain will implement `from_str` in the future,
/// // currently it doesn't have a validator/parser.
/// let domain = Domain::from_unchecked("example.com".to_owned());
/// // Normally you create this _once per application_.
/// let ctx = simple_context::new(domain, "xqi93".parse().unwrap())
///     .unwrap();
///
/// let mut mail = Mail::plain_text("Hy there!").unwrap();
/// mail.set_headers(headers! {
///     _From: [("I'm Awesome", "bla@examle.com")],
///     _To: ["unknow@example.com"],
///     Subject: "Hy there message"
/// }.unwrap()).unwrap();
///
/// // We don't added anythink which needs loading but we could have
/// // and all of it would have been loaded concurrent and async.
/// let encoded = mail.into_encodeable_mail(ctx.clone())
///     .wait().unwrap()
///     .encode_into_bytes(MailType::Ascii).unwrap();
///
/// let mail_str = str::from_utf8(&encoded).unwrap();
/// println!("{}", mail_str);
/// # }
/// ```
///
/// And here is an example to create the same mail using the
/// builder:
///
/// ```
/// # extern crate mail_types;
/// # #[macro_use] extern crate mail_headers as headers;
/// // either from `mail::headers` or from `mail_header as headers`
/// use headers::*;
/// use mail_types::{Mail, Builder, Resource};
///
/// # fn main() {
/// let resource = Resource::sourceless_from_string("Hy there!");
/// let mail = Builder::singlepart(resource)
///     .headers(headers! {
///         _From: [("I'm Awesome", "bla@examle.com")],
///         _To: ["unknow@example.com"],
///         Subject: "Hy there message"
///     }.unwrap()).unwrap()
///     .build().unwrap();
/// # }
/// ```
///
/// And here is an example creating a multipart mail
/// with a made up `multipart` type.
///
/// ```
/// # extern crate mail_types;
/// # #[macro_use] extern crate mail_headers as headers;
/// // either from `mail::headers` or from `mail_header as headers`
/// use headers::*;
/// use mail_types::{Mail, Builder, Resource, mime::gen_multipart_media_type};
///
/// # fn main() {
/// let sub_body1 = Mail::plain_text("Body 1").unwrap();
/// let sub_body2 = Mail::plain_text("Body 2, yay").unwrap();
///
/// // This will generate `multipart/x.made-up-think; boundary=randome_generate_boundary`
/// let media_type = gen_multipart_media_type("x.made-up-thing").unwrap();
/// let mail = Builder::multipart(media_type).unwrap()
///     .body(sub_body1).unwrap()
///     .body(sub_body2).unwrap()
///     .headers(headers! {
///         _From: [("I'm Awesome", "bla@examle.com")],
///         _To: ["unknow@example.com"],
///         Subject: "Hy there message"
///     }.unwrap()).unwrap()
///     .build().unwrap();
/// # }
/// ```
#[derive(Clone, Debug)]
pub struct Mail {
    headers: HeaderMap,
    body: MailPart,
}

/// A type which either represents a single body, or multiple modies.
///
/// Note that you could have a mime multipart body just containing a
/// single body _and_ it being semantically important to be this way,
/// so we have to differ between both kinds (instead of just having
/// a `Vec` of mails)
#[derive(Clone, Debug)]
pub enum MailPart {
    SingleBody {
        body: Resource
    },
    MultipleBodies {
        //TODO[now]: use Vec1
        bodies: Vec<Mail>,
        /// This is part of the standard! But we won't
        /// make it public available for now. Through
        /// there is a chance that we need to do so
        /// in the future as some mechanisms might
        /// misuse this, well unusual think.
        hidden_text: SoftAsciiString
    }
}

/// A future resolving to an encodeable mail.
///
/// The future resolves like this:
///
/// 1. it makes sure all contained futures are resolved, i.e. all
///    `Resources` are loaded and transfer encoded if needed
/// 2. it inserts auto-generated headers, i.e. `Content-Type`,
///    `Content-Transfer-Encoding` are generated and `Date`, too if
///    it is needed.
///     1. If needed it generates a boundary for the `Content-Type`
/// 3. contextual validators are used (including a check if there is
///    a `From` header)
/// 4. as the mail is now ready to be encoded it resolves to an
///    `EncodableMail`
///
/// # Error (while resolving the future)
///
/// - if one of the contained futures fails, e.g. if a resource can not
///   be loaded or encoded
/// - if a contextual validator fails, e.g. `From` header is missing or
///   there is a multi mailbox `From` header but no `Sender` header
///
pub struct MailFuture<T: Context> {
    mail: Option<Mail>,
    ctx: T,
    inner: future::JoinAll<Vec<ResourceLoadingFuture<T>>>,
}

/// a mail with all contained futures resolved, so that it can be encoded
//#[derive(Clone)]
pub struct EncodableMail(Mail, Vec<ResourceAccessGuard>);

impl Mail {

    pub fn plain_text(text: impl Into<String>) -> Result<Self, BuilderError> {
        let resource = Resource::sourceless_from_string(text);
        Builder::singlepart(resource).build()
    }

    /// Add a new header, converting the header component `comp` to the right type.
    ///
    /// Note that some headers namely `Content-Transfer-Encoding` and
    /// for singlepart mails `Content-Type` are derived from the content
    /// and _cannot_ be set. Also `Date` is auto-generated if not set.
    ///
    /// Be aware that while the most know headers (like `From`) should only
    /// appear one time others can appear multiple times. This function just
    /// adds another header. It doesn't check if a header with the same name
    /// was already set _and_ if the header should only appear one time.
    ///
    /// # Error
    ///
    /// An error is returned if:
    ///
    /// - A `Content-Type` header is set in for a single part mail, or
    ///   a `Content-Type` header which is not `multipart` is set for an
    ///   multipart mail.
    ///
    /// - A `Content-Transfer-Encoding` header is set
    ///
    pub fn set_header<H, C>(&mut self, header: H, comp: C)
        -> Result<(), BuilderError>
        where H: Header,
              H::Component: EncodableInHeader,
              C: HeaderTryInto<H::Component>
    {
        let comp = comp.try_into()?;
        check_header::<H>(&comp, self.body.is_multipart())?;
        self.headers.insert(header, comp)?;
        Ok(())
    }

    /// Sets all header from the provided header map.
    pub fn set_headers(&mut self, headers: HeaderMap)
        -> Result<(), BuilderError>
    {
        check_multiple_headers(&headers, self.body.is_multipart())?;
        self.headers.combine(headers);
        Ok(())
    }

    /// Returns a reference to the currently set headers.
    ///
    /// Note that some headers namely `Content-Transfer-Encoding` and
    /// for singlepart mails `Content-Type` are derived from the content
    /// and _cannot_ be set. Also `Date` is auto-generated if not set.
    pub fn headers(&self) -> &HeaderMap {
        &self.headers
    }

    /// Returns a reference to the body/bodies.
    pub fn body(&self) -> &MailPart {
        &self.body
    }

    //TODO potentially change it into as_encodable_mail(&mut self)
    /// Turns the mail into a future with resolves to an `EncodeableMail`
    ///
    /// Use this if you want to encode a mail. This is needed as `Resource`
    /// instances used in the mail are loaded "on-demand", i.e. if you attach
    /// two images but never turn the mail into an encodable mail the images
    /// are never loaded from disk.
    pub fn into_encodeable_mail<C: Context>(self, ctx: C) -> MailFuture<C> {
        let mut futures = Vec::new();
        //FIXME[rust/! type]: use ! instead of (),
        // alternatively use futures::Never if futures >= 0.2
        self.walk_mail_bodies::<_, ()>(&mut |resource: &Resource| {
            Ok(futures.push(resource.create_loading_future(ctx.clone())))
        }).unwrap();

        MailFuture {
            ctx,
            inner: future::join_all(futures),
            mail: Some(self)
        }
    }

    fn walk_mail_bodies<FN, E>(&self, use_it_fn: &mut FN) -> Result<(), E>
        where FN: FnMut(&Resource) -> Result<(), E>
    {
        use self::MailPart::*;
        match self.body {
            SingleBody { ref  body } =>
                use_it_fn(body)?,
            MultipleBodies { ref  bodies, .. } =>
                for body in bodies {
                    body.walk_mail_bodies(use_it_fn)?
                }
        }
        Ok(())
    }
}

impl MailPart {

    /// Returns `true` if it's an multipart body.
    pub fn is_multipart(&self) -> bool {
        use self::MailPart::*;
        match *self {
            SingleBody { .. } => false,
            MultipleBodies { .. } => true
        }
    }
}


impl<T> Future for MailFuture<T>
    where T: Context,
{
    type Item = EncodableMail;
    type Error = MailError;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        let anti_unload_guards = try_ready!(self.inner.poll());
        let mail = self.mail.take().unwrap();
        let enc_mail = EncodableMail::from_loaded_mail(mail, anti_unload_guards, &self.ctx)?;
        Ok(Async::Ready(enc_mail))
    }
}

impl EncodableMail {

    /// Encode the mail using the given encoding buffer.
    ///
    /// After encoding succeeded the buffer should contain
    /// a fully encoded mail including all attachments, embedded
    /// images alternate bodies etc.
    ///
    /// # Error
    ///
    /// This can fail for a large number of reasons, e.g. some
    /// input can not be encoded with the given mail type or
    /// some headers/resources breack the mails hard line length limit.
    pub fn encode(&self, encoder: &mut EncodingBuffer) -> Result<(), MailError> {
        encode::encode_mail(self, true, encoder)
    }

    /// A wrapper for `encode` which will create a buffer, enocde the mail and then returns the buffers content.
    pub fn encode_into_bytes(&self, mail_type: MailType) -> Result<Vec<u8>, MailError> {
        let mut buffer = EncodingBuffer::new(mail_type);
        self.encode(&mut buffer)?;
        Ok(buffer.into())
    }

    fn from_loaded_mail(
        mut mail: Mail,
        anti_unload_guards: Vec<ResourceAccessGuard>,
        ctx: &impl Context
    )
        -> Result<Self, MailError>
    {
        recursively_insert_generated_headers(&mut mail)?;

        auto_gen_top_level_only_headers(&mut mail.headers, ctx)?;

        check_required_headers(&mail.headers)?;

        mail.headers.use_contextual_validators()?;

        Ok(EncodableMail(mail, anti_unload_guards))
    }
}

/// inserts ContentType and ContentTransferEncoding into
/// the headers of any contained `MailPart::SingleBody`,
/// based on the `Resource` representing the body
fn recursively_insert_generated_headers(mail: &mut Mail) -> Result<(), MailError> {
    match mail.body {
        MailPart::SingleBody { ref body } => {
           auto_gen_headers(&mut mail.headers, body)?;
        }
        MailPart::MultipleBodies { ref mut bodies, .. } => {
            for sub_mail in bodies {
                recursively_insert_generated_headers(sub_mail)?;
            }
        }

    }
    Ok(())
}

/// check if headers which are generally required are in the header map
///
/// Normally constraints are checked through the validators, but this won't
/// work if there is no guarantee the validator was inserted. This only applies
/// for `Date`, `From` as they have to appear once no matter what. But `Date`
/// is auto generated so only `From` is checked here.
/// The only required headers are `From` and `Date`, all other quantitative
/// constraints can be checked with contextual validators,
fn check_required_headers(headers: &HeaderMap) -> Result<(), MailError> {
    if headers.contains(_From) {
        Ok(())
    } else {
        Err(HeaderValidationError::from(BuildInValidationError::NoFrom).into())
    }
}

fn auto_gen_headers(headers: &mut HeaderMap, body: &Resource) -> Result<(), MailError> {
    let file_buffer = body.get_if_encoded()
        .expect("[BUG] encoded mail, should only contain already transferencoded resources");

    headers.insert(ContentType, file_buffer.content_type().clone())?;
    headers.insert(ContentTransferEncoding, file_buffer.transfer_encoding().clone())?;
    Ok(())
}

fn auto_gen_top_level_only_headers(headers: &mut HeaderMap, ctx: &impl Context)
    -> Result<(), MailError>
{
    if !headers.contains(Date) {
        headers.insert(Date, DateTime::now())?;
    }

    if !headers.contains(MessageId) {
        headers.insert(MessageId, ctx.generate_message_id())?;
    }

    Ok(())
}

impl Deref for EncodableMail {

    type Target = Mail;
    fn deref( &self ) -> &Self::Target {
        &self.0
    }
}

impl Into<Mail> for EncodableMail {
    fn into( self ) -> Mail {
        self.0
    }
}

impl fmt::Debug for EncodableMail {
    fn fmt(&self, fter: &mut fmt::Formatter) -> fmt::Result {
        write!(fter, "EncodableMail {{ .. }}")
    }
}


#[cfg(test)]
mod test {
    use std::fmt::Debug;
    use headers::components::MediaType;
    use ::file_buffer::FileBuffer;
    use ::Resource;

    fn resource_from_text<I: Into<String>>(text: I) -> Resource {
        let text: String = text.into();
        let mt = MediaType::parse("text/plain; charset=utf-8").unwrap();
        let fb = FileBuffer::new(mt, text.into());
        Resource::sourceless_from_buffer(fb)
    }

    trait AssertDebug: Debug {}
    trait AssertSend: Send {}
    trait AssertSync: Sync {}

    mod Mail {
        #![allow(non_snake_case)]
        use std::str;
        use headers::components::TransferEncoding;
        use headers::{
            Subject, Comments
        };
        use default_impl::test_context;
        use super::super::*;
        use super::resource_from_text;
        use super::{AssertDebug, AssertSend, AssertSync};

        fn load_blocking<C>(r: &Resource, ctx: &C) -> ResourceAccessGuard
            where C: Context
        {
            r.create_loading_future(ctx.clone()).wait().unwrap()
        }

        impl AssertDebug for Mail {}
        impl AssertSend for Mail {}
        impl AssertSync for Mail {}


        #[test]
        fn walk_mail_bodies_does_not_skip() {
            let mail = Mail {
                headers: HeaderMap::new(),
                body: MailPart::MultipleBodies {
                    bodies: vec! [
                        Mail {
                            headers: HeaderMap::new(),
                            body: MailPart::MultipleBodies {
                                bodies: vec! [
                                    Mail {
                                        headers: HeaderMap::new(),
                                        body: MailPart::SingleBody {
                                            body: resource_from_text("r1")
                                        }
                                    },
                                    Mail {
                                        headers: HeaderMap::new(),
                                        body: MailPart::SingleBody {
                                            body: resource_from_text("r2")
                                        }
                                    }
                                ],
                                hidden_text: Default::default()
                            }
                        },
                        Mail {
                            headers: HeaderMap::new(),
                            body: MailPart::SingleBody {
                                body: resource_from_text("r3")
                            }
                        }

                    ],
                    hidden_text: Default::default()
                }
            };

            let ctx = test_context();
            let mut body_count = 0;
            mail.walk_mail_bodies::<_, ()>(&mut |body: &Resource| {
                body_count += 1;
                let access = load_blocking(body, &ctx);
                let _encoded0 = access.access();
                let encoded = body.get_if_encoded().expect("it should be loaded");
                let slice = str::from_utf8(&encoded[..]).unwrap();
                assert!([ "r1", "r2", "r3"].contains(&slice));
                Ok(())
            }).unwrap();
        }

        #[test]
        fn walk_mail_bodies_handles_errors() {
            let mail = Mail {
                headers: HeaderMap::new(),
                body: MailPart::SingleBody {
                    body: resource_from_text("r0"),
                }
            };
            assert_ok!(mail.walk_mail_bodies::<_, ()>(&mut |_| { Ok(()) }));
            assert_err!(mail.walk_mail_bodies::<_, ()>(&mut |_| { Err(()) }));
        }

        #[test]
        fn set_header_checks_the_header() {
            let mut mail = Mail {
                headers: HeaderMap::new(),
                body: MailPart::SingleBody {
                    body: resource_from_text("r0"),
                }
            };

            assert_err!(
                mail.set_header(ContentTransferEncoding, TransferEncoding::Base64));
            //Note: a more fine grained test is done in ::mail::builder::test
            assert_err!(mail.set_header(ContentType, "text/plain"));
            assert_err!(mail.set_header(ContentType, "multipart/plain"));
        }

        #[test]
        fn set_header_set_a_header() {
            let mut mail = Mail {
                headers: HeaderMap::new(),
                body: MailPart::SingleBody {
                    body: resource_from_text("r0"),
                }
            };
            assert_ok!(mail.set_header(Subject, "hy"));
            assert!(mail.headers().contains(Subject));
        }

        #[test]
        fn set_headers_checks_the_headers() {
            let mut mail = Mail {
                headers: HeaderMap::new(),
                body: MailPart::SingleBody {
                    body: resource_from_text("r0"),
                }
            };
            assert_err!(mail.set_headers(headers! {
                ContentType: "test/html;charset=utf8"
            }.unwrap()));
        }

        #[test]
        fn set_headers_sets_all_headers() {
            let mut mail = Mail {
                headers: HeaderMap::new(),
                body: MailPart::SingleBody {
                    body: resource_from_text("r0"),
                }
            };
            assert_ok!(mail.set_headers(headers! {
                Subject: "yes",
                Comments: "so much"
            }.unwrap()));

            assert!(mail.headers().contains(Subject));
            assert!(mail.headers().contains(Comments));
        }

    }

    mod EncodableMail {
        #![allow(non_snake_case)]
        use chrono::{Utc, TimeZone};
        use headers::components::{
            TransferEncoding,
            DateTime
        };
        use headers::{
            _From, ContentType, ContentTransferEncoding,
            Date, Subject
        };
        use default_impl::test_context;
        use super::super::*;
        use super::resource_from_text;
        use super::{AssertDebug, AssertSend, AssertSync};

        impl AssertDebug for EncodableMail {}
        impl AssertSend for EncodableMail {}
        impl AssertSync for EncodableMail {}

        #[test]
        fn sets_generated_headers_for_outer_mail() {
            let resource = resource_from_text("r9");
            let mail = Mail {
                headers: headers!{
                    _From: ["random@this.is.no.mail"],
                    Subject: "hoho"
                }.unwrap(),
                body: MailPart::SingleBody { body: resource }
            };

            let ctx = test_context();
            let enc_mail = assert_ok!(mail.into_encodeable_mail(ctx).wait());

            let headers: &HeaderMap = enc_mail.headers();
            assert!(headers.contains(_From));
            assert!(headers.contains(Subject));
            assert!(headers.contains(Date));
            assert!(headers.contains(ContentType));
            assert!(headers.contains(ContentTransferEncoding));
            assert!(headers.contains(MessageId));
            assert_eq!(headers.len(), 6);

            let res = headers.get_single(ContentType)
                .unwrap()
                .unwrap();

            assert_eq!(res.as_str_repr(), "text/plain; charset=utf-8");

            let res = headers.get_single(ContentTransferEncoding)
                .unwrap()
                .unwrap();

            assert_eq!(res, &TransferEncoding::QuotedPrintable);
        }

        #[test]
        fn sets_generated_headers_for_sub_mails() {
            let resource = resource_from_text("r9");
            let mail = Mail {
                headers: headers!{
                    _From: ["random@this.is.no.mail"],
                    Subject: "hoho",
                    ContentType: "multipart/mixed"
                }.unwrap(),
                body: MailPart::MultipleBodies {
                    bodies: vec![
                        Mail {
                            headers: HeaderMap::new(),
                            body: MailPart::SingleBody { body: resource }
                        }
                    ],
                    hidden_text: Default::default()
                }
            };

            let ctx = test_context();
            let mail = mail.into_encodeable_mail(ctx).wait().unwrap();

            assert!(mail.headers().contains(_From));
            assert!(mail.headers().contains(Subject));
            assert!(mail.headers().contains(Date));
            //the Builder would have set it but as we didn't use it (intentionally) it's not set
            //assert!(headers.contains(ContentType));

            if let MailPart::MultipleBodies { ref bodies, ..} = mail.body {
                let headers = bodies[0].headers();
                assert_not!(headers.contains(Date));

                let res = headers.get_single(ContentType)
                    .unwrap()
                    .unwrap();

                assert_eq!(res.as_str_repr(), "text/plain; charset=utf-8");

                let res = headers.get_single(ContentTransferEncoding)
                    .unwrap()
                    .unwrap();

                assert_eq!(res, &TransferEncoding::QuotedPrintable);

            } else {
                unreachable!()
            }
        }

        #[test]
        fn runs_contextual_validators() {
            let mail = Mail {
                headers: headers!{
                    _From: ["random@this.is.no.mail", "u.p.s@s.p.u"],
                    Subject: "hoho"
                }.unwrap(),
                body: MailPart::SingleBody { body: resource_from_text("r9") }
            };

            let ctx = test_context();
            assert_err!(mail.into_encodeable_mail(ctx).wait());
        }

        #[test]
        fn checks_there_is_from() {
            let mail = Mail {
                headers: headers!{
                    Subject: "hoho"
                }.unwrap(),
                body: MailPart::SingleBody { body: resource_from_text("r9") }
            };

            let ctx = test_context();
            assert_err!(mail.into_encodeable_mail(ctx).wait());
        }

        #[test]
        fn does_not_override_date_if_set() {
            let provided_date = Utc.ymd(1992, 5, 25).and_hms(23, 41, 12);
            let mail = Mail {
                headers: headers!{
                    _From: ["random@this.is.no.mail"],
                    Subject: "hoho",
                    Date: DateTime::new(provided_date.clone())
                }.unwrap(),
                body: MailPart::SingleBody { body: resource_from_text("r9") }
            };

            let ctx = test_context();
            let enc_mail = assert_ok!(mail.into_encodeable_mail(ctx).wait());
            let used_date = enc_mail.headers()
                .get_single(Date)
                .unwrap()
                .unwrap();

            assert_eq!(**used_date, provided_date);


        }

    }

}