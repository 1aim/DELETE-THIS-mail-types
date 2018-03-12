use std::ops::Deref;
use std::fmt;

use core::codec::{EncodableInHeader, Encoder, Encodable};
use soft_ascii_string::SoftAsciiString;
use futures::{ future, Future, Async, Poll };

use core::error::{Result, Error};
use core::utils::HeaderTryInto;
use core::header::{Header, HeaderMap};
use mheaders::{
    ContentType, From,
    ContentTransferEncoding,
    Date, MessageId
};
use mheaders::components::DateTime;
use context::BuilderContext;

use self::builder::{
    check_header,
    check_multiple_headers,
};

pub use self::builder::{
    Builder, MultipartBuilder, SinglepartBuilder
};

pub use self::resource::*;


pub mod context;
mod resource;
mod builder;
mod encode;


#[derive(Debug)]
pub struct Mail {
    //NOTE: by using some OwnedOrStaticRef AsciiStr we can probably safe a lot of
    // unnecessary allocations
    headers: HeaderMap,
    body: MailPart,
}

#[derive(Debug)]
pub enum MailPart {
    SingleBody {
        body: Resource
    },
    MultipleBodies {
        bodies: Vec<Mail>,
        hidden_text: SoftAsciiString
    }
}

/// a future resolving to an encodeable mail
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
pub struct MailFuture<T: BuilderContext> {
    mail: Option<Mail>,
    inner: future::JoinAll<Vec<ResourceLoadingFuture<T>>>,
}

/// a mail with all contained futures resolved, so that it can be encoded
pub struct EncodableMail(Mail, Vec<ResourceAccessGuard>);

impl Mail {


    /// adds a new header,
    ///
    /// - if the header already existed, the existing one will be overriden and the
    ///   old header will be returned
    /// - `Content-Transfer-Encoding` it might be overwritten later one
    ///
    /// # Failure
    ///
    /// if a Content-Type header is set, which conflicts with the body, mainly if
    /// you set a multipart content type on a non-multipart body or the other way around
    ///
    pub fn set_header<H, C>( &mut self, header: H, comp: C) -> Result<()>
        where H: Header,
              H::Component: EncodableInHeader,
              C: HeaderTryInto<H::Component>
    {
        let comp = comp.try_into()?;
        check_header::<H>( &comp, self.body.is_multipart() )?;
        self.headers.insert( header, comp )?;
        Ok( () )
    }

    pub fn set_headers( &mut self, headers: HeaderMap ) -> Result<()> {
        check_multiple_headers( &headers, self.body.is_multipart() )?;
        self.headers.try_extend( headers )?;
        Ok( () )
    }

    pub fn headers( &self ) -> &HeaderMap {
        &self.headers
    }

    pub fn body( &self ) -> &MailPart {
        &self.body
    }

    //TODO potentially change it into as_encodable_mail(&mut self)
    /// Turns the mail into a future with resolves to an `EncodeableMail`
    ///
    pub fn into_encodeable_mail<C: BuilderContext>(self, ctx: &C ) -> MailFuture<C> {
        let mut futures = Vec::new();
        self.walk_mail_bodies(&mut |resource: &Resource| {
            Ok(futures.push(resource.create_loading_future(ctx.clone())))
        }).unwrap();

        MailFuture {
            inner: future::join_all(futures),
            mail: Some( self )
        }
    }

    fn walk_mail_bodies<FN>( &self, use_it_fn: &mut FN) -> Result<()>
        where FN: FnMut( &Resource ) -> Result<()>
    {
        use self::MailPart::*;
        match self.body {
            SingleBody { ref  body } =>
                use_it_fn( body )?,
            MultipleBodies { ref  bodies, .. } =>
                for body in bodies {
                    body.walk_mail_bodies( use_it_fn )?
                }
        }
        Ok( () )
    }
}







impl MailPart {

    pub fn is_multipart( &self ) -> bool {
        use self::MailPart::*;
        match *self {
            SingleBody { .. } => false,
            MultipleBodies { .. } => true
        }
    }
}


impl<T> Future for MailFuture<T>
    where T: BuilderContext,
{
    type Item = EncodableMail;
    type Error = Error;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        use std::convert::From;
        let anti_unload_guards = try_ready!(self.inner.poll());
        let mail = self.mail.take().unwrap();
        let enc_mail = EncodableMail::from_loaded_mail(mail, anti_unload_guards)?;
        Ok(Async::Ready(enc_mail))
    }
}

impl EncodableMail {

    fn from_loaded_mail(mut mail: Mail, anti_unload_guards: Vec<ResourceAccessGuard>) -> Result<Self> {
        insert_generated_headers(&mut mail)?;
        // also insert `Date` if needed, but only on the outer most header map
        if !mail.headers.contains(Date) {
            mail.headers.insert(Date, DateTime::now())?;
        }

        mail.headers.use_contextual_validators()?;

        // also check `From` only on the outer most header map
        if !mail.headers.contains(From) {
            bail!("mail must have a `From` header");
        }
        if !mail.headers.contains(MessageId) {
            warn!("mail without MessageId")
        }

        Ok(EncodableMail(mail, anti_unload_guards))
    }
}

/// inserts ContentType and ContentTransferEncoding into
/// the headers of any contained `MailPart::SingleBody`,
/// based on the `Resource` representing the body
fn insert_generated_headers(mail: &mut Mail) -> Result<()> {
    match mail.body {
        MailPart::SingleBody { ref body } => {
           auto_gen_headers(&mut mail.headers, body)?;
        }
        MailPart::MultipleBodies { ref mut bodies, .. } => {
            for sub_mail in bodies {
                insert_generated_headers(sub_mail)?;
            }
        }

    }
    Ok(())
}

fn auto_gen_headers(headers: &mut HeaderMap, body: &Resource) -> Result<()> {
    let file_buffer = body.get_if_encoded()
        .expect("[BUG] encoded mail, should only contain already transferencoded resources");

    headers.insert(ContentType, file_buffer.content_type().clone())?;
    headers.insert(ContentTransferEncoding, file_buffer.transfer_encoding().clone())?;
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

impl Encodable<Resource> for EncodableMail {

    fn encode(&self, encoder:  &mut Encoder<Resource>) -> Result<()> {
        // does not panic as a EncodableMail only is constructed from
        // a Mail which has all of it's bodies resolved, without failure
        encode::encode_mail( &self, true, encoder )
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
    use ::MediaType;
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
        use mheaders::components::TransferEncoding;
        use mheaders::{
            Subject, Comments
        };
        use default_impl::test_context;
        use super::super::*;
        use super::resource_from_text;
        use super::{AssertDebug, AssertSend, AssertSync};

        fn load_blocking<C>(r: &Resource, ctx: &C) -> ResourceAccessGuard
            where C: BuilderContext
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
            mail.walk_mail_bodies(&mut |body: &Resource| {
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
            assert_ok!(mail.walk_mail_bodies(&mut |_| { Ok(()) }));
            assert_err!(mail.walk_mail_bodies(&mut |_| { bail!("bad") }));
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
        use mheaders::components::{
            TransferEncoding,
            DateTime
        };
        use mheaders::{
            From, ContentType, ContentTransferEncoding,
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
                    From: ["random@this.is.no.mail"],
                    Subject: "hoho"
                }.unwrap(),
                body: MailPart::SingleBody { body: resource }
            };

            let ctx = test_context();
            let enc_mail = assert_ok!(mail.into_encodeable_mail(&ctx).wait());

            let headers: &HeaderMap = enc_mail.headers();
            assert!(headers.contains(From));
            assert!(headers.contains(Subject));
            assert!(headers.contains(Date));
            assert!(headers.contains(ContentType));
            assert!(headers.contains(ContentTransferEncoding));

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
                    From: ["random@this.is.no.mail"],
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
            let mail = mail.into_encodeable_mail(&ctx).wait().unwrap();

            assert!(mail.headers().contains(From));
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
                    From: ["random@this.is.no.mail", "u.p.s@s.p.u"],
                    Subject: "hoho"
                }.unwrap(),
                body: MailPart::SingleBody { body: resource_from_text("r9") }
            };

            let ctx = test_context();
            assert_err!(mail.into_encodeable_mail(&ctx).wait());
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
            assert_err!(mail.into_encodeable_mail(&ctx).wait());
        }

        #[test]
        fn does_not_override_date_if_set() {
            let provided_date = Utc.ymd(1992, 5, 25).and_hms(23, 41, 12);
            let mail = Mail {
                headers: headers!{
                    From: ["random@this.is.no.mail"],
                    Subject: "hoho",
                    Date: DateTime::new(provided_date.clone())
                }.unwrap(),
                body: MailPart::SingleBody { body: resource_from_text("r9") }
            };

            let ctx = test_context();
            let enc_mail = assert_ok!(mail.into_encodeable_mail(&ctx).wait());
            let used_date = enc_mail.headers()
                .get_single(Date)
                .unwrap()
                .unwrap();

            assert_eq!(**used_date, provided_date);


        }

    }

}