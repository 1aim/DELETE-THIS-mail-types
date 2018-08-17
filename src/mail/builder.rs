use soft_ascii_string::SoftAsciiString;

use media_type::BOUNDARY;

use common::utils::uneraser_ref;
use common::encoder::EncodableInHeader;
use headers::{
    HeaderTryInto,
    Header, HeaderMap,
    ContentType,
    ContentTransferEncoding
};
use headers::error::HeaderTypeError;
use headers::components::MediaType;

use ::error::{BuilderError, OtherBuilderErrorKind};
use ::mime::create_random_boundary;

use super::resource::Resource;
use super::{ MailPart, Mail };

/// Basic builder type, this is just an entry point to get one of the "real" builders.
///
/// - use `Builder::multipart` to get a builder for a multi part mime body
/// - use `Builder::singlepart` to get a builder for a single part mime body/non mime mail body
pub struct Builder;

struct BuilderShared {
    headers: HeaderMap
}

/// Builder used to build a mail body based on a Resource.
///
/// This is used for everything which is not a multi part
/// mime body and is used to build the "leaf" bodies of
/// a multi part mime body.
pub struct SinglepartBuilder {
    inner: BuilderShared,
    body: Resource
}

/// Builder used to build a multi part mime mail body.
pub struct MultipartBuilder {
    inner: BuilderShared,
    hidden_text: Option<SoftAsciiString>,
    bodies: Vec<Mail>
}

impl BuilderShared {

    fn new() -> Self {
        BuilderShared {
            headers: HeaderMap::new()
        }
    }


    ///
    /// # Error
    ///
    /// A error is returned if the header is incompatible with this builder,
    /// i.e. if a ContentType header is set with a non-multipart content type
    /// is set on a multipart mail or a multipart content type is set on a
    /// non-mutltipart mail
    ///
    /// NOTE: do NOT add other error cases
    fn header<H>(
        &mut self,
        header: H,
        hbody: H::Component,
        is_multipart: bool
    ) -> Result<usize, BuilderError>
        where H: Header,
              H::Component: EncodableInHeader
    {
        check_header::<H>(&hbody, is_multipart)?;
        Ok(self.headers.insert(header, hbody)?)
    }

    /// might already have added some headers even if it returns Err(...)
    fn headers(&mut self, headers: HeaderMap, is_multipart: bool)
        -> Result<(), BuilderError>
    {
        //TODO CONSIDER:
        // it is not impossible to make this function "transactional" for HeaderMap
        // (it is impossible for TotalOrderMultiMap) by:
        // 1. implement pop on TotalOrderMultiMap
        // 2. store current len before extending
        // 3. pop until the stored length is reached again
        check_multiple_headers(&headers, is_multipart)?;
        self.headers.combine(headers);
        Ok(())
    }

    fn build(self, body: MailPart) -> Result<Mail, BuilderError> {
        Ok(Mail {
            headers: self.headers,
            body: body,
        })
    }
}

pub(crate) fn check_multiple_headers(headers: &HeaderMap , is_multipart: bool)
     -> Result<(), BuilderError>
{
    if let Some( .. ) = headers.get_single(ContentTransferEncoding) {
        return Err(OtherBuilderErrorKind::InsertingContentTransferEncodingHeader.into());
    }
    //FIMXE[BUG] get->is_multipart seems wrong instead is_multipart->get?
    if let Some( mime ) = headers.get_single(ContentType) {
        if is_multipart {
            if !mime?.is_multipart() {
                return Err(OtherBuilderErrorKind::SingleMultipartMixup.into());
            }
        } else {
            return Err(OtherBuilderErrorKind::InsertSinglepartContentTypeHeader.into());
        }
    }
    Ok( () )
}

pub(crate) fn check_header<H>(
    hbody: &H::Component,
    is_multipart: bool
) -> Result<(), BuilderError>
    where H: Header,
          H::Component: EncodableInHeader
{
    match H::name().as_str() {
        "Content-Type" => {
            if is_multipart {
                let mime: &MediaType = uneraser_ref(hbody)
                    .ok_or_else(|| HeaderTypeError::new(ContentType::name()))?;
                if !mime.is_multipart() {
                    return Err(OtherBuilderErrorKind::SingleMultipartMixup.into());
                }
            } else {
                return Err(OtherBuilderErrorKind::InsertSinglepartContentTypeHeader.into());
            }

        },
        "Content-Transfer-Encoding" => {
            return Err(OtherBuilderErrorKind::InsertingContentTransferEncodingHeader.into());
        }
        _ => {}
    }
    Ok( () )
}

impl Builder {

    /// Create a MultipartBuilder with the given media-type as content-type.
    ///
    /// This function will always set the boundary parameter to a random
    /// generated boundary string. If the media type already had it
    /// boundary parameter it is overwritten.
    ///
    /// # Error
    ///
    /// if the media-type is not a `multipart/` media type an
    /// error is returned
    pub fn multipart(media_type: MediaType) -> Result<MultipartBuilder, BuilderError> {
        if !media_type.is_multipart() {
            return Err(BuilderError::from(OtherBuilderErrorKind::SingleMultipartMixup));
        }

        let mut media_type = media_type;
        let boundary = create_random_boundary();
        media_type.set_param(BOUNDARY, boundary);

        let res = MultipartBuilder {
            inner: BuilderShared::new(),
            hidden_text: None,
            bodies: Vec::new(),
        };

        //UNWRAP_SAFETY: it can only fail with illegal headers,
        // but this header can not be illegal
        Ok(res.header(ContentType, media_type).unwrap())
    }

    /// Create a builder for a non multipart mail body based on a resource.
    ///
    /// This can be for example an image or a text or html. Its also
    /// used inside a multipart mime body for the "leaf" bodies.
    pub fn singlepart(r: Resource) -> SinglepartBuilder {
        SinglepartBuilder {
            inner: BuilderShared::new(),
            body: r,
        }
    }

}

impl SinglepartBuilder {

    /// Add a header to the body.
    ///
    /// Be aware that this body isn't necessary used as a top level
    /// body, so adding headers which are meant for the mail (and not this
    /// specific body) is discouraged.
    ///
    /// # Error
    ///
    /// If the headers content/component `hbody` can not be converted
    /// in the type required by the header `header` an error is returned.
    /// This can for example happen when adding a header requiring a mailbox
    /// or email and passing in a (malformed) email as string.
    ///
    /// If the header is `Content-Type` or `Content-Transfer-Encoding` an
    /// error is returned as they are generated based on the resource.
    pub fn header<H, C>(
        mut self,
        header: H,
        hbody: C
    ) -> Result<Self, BuilderError>
        where H: Header,
              H::Component: EncodableInHeader,
              C: HeaderTryInto<H::Component>
    {
        let comp = hbody.try_into()?;
        self.inner.header(header, comp, false)?;
        Ok(self)
    }

    /// Add all headers from the given header map into this builder.
    ///
    /// Be aware that validation tasks like "check if there is only
    /// on `Content-Id` header" are _not_ automatically run every
    /// time an header is added (as this would be a problem with
    /// some edge cases).
    ///
    /// # Error
    ///
    /// If the header is `Content-Type` or `Content-Transfer-Encoding` an
    /// error is returned as they are generated based on the resource.
    pub fn headers(mut self, headers: HeaderMap) -> Result<Self, BuilderError> {
        self.inner.headers(headers, false)?;
        Ok(self)
    }

    /// Convert the header into a `Mail` type.
    ///
    /// The returned mail is as such not a multipart mime mail
    /// but e.g. a plain text mail.
    ///
    //TODO[NOW] remove result it never errors
    pub fn build(self) -> Result<Mail, BuilderError> {
        self.inner.build( MailPart::SingleBody { body: self.body } )
    }
}

impl MultipartBuilder {

    /// Add a header to the body.
    ///
    /// Be aware that this body isn't necessary used as a top level
    /// body.
    ///
    /// # Error
    ///
    /// An error is returned if:
    ///
    /// - A `Content-Type` header is added with a media type which
    ///   is not `multipart`.
    ///
    /// - A `Content-Transfer-Encoding` header is added.
    ///
    pub fn header<H, C>(
        mut self,
        header: H,
        hbody: C
    ) -> Result<Self, BuilderError>
        where H: Header,
              H::Component: EncodableInHeader,
              C: HeaderTryInto<H::Component>
    {
        let comp = hbody.try_into()?;
        self.inner.header(header, comp, true)?;
        Ok(self)
    }

    /// Add all headers from the given header map into this builder.
    pub fn headers(mut self, headers: HeaderMap) -> Result<Self, BuilderError> {
        self.inner.headers(headers, true)?;
        Ok(self)
    }

    /// Add a new (sub) body.
    ///
    /// The new body is a mail instance, which could have been created
    /// by the `SinglepartBuilder` or this builder. So it can be
    /// "just some content" (e.g. a plain/text body) or another
    /// multipart body.
    //TODO[NOW]: remove result
    pub fn body(mut self, body: Mail) -> Result<Self, BuilderError> {
        self.bodies.push(body);
        Ok(self)
    }

    /// Builds a mail with a multipart mime.
    ///
    /// # Error
    ///
    /// This fails if:
    ///
    /// - Not at last one body was added.
    ///
    pub fn build(self) -> Result<Mail, BuilderError> {
        if self.bodies.len() == 0 {
            Err(BuilderError::from(OtherBuilderErrorKind::EmptyMultipartBody))
        } else {
            self.inner.build(MailPart::MultipleBodies {
                bodies: self.bodies,
                hidden_text: self.hidden_text.unwrap_or(SoftAsciiString::new()),
            })
        }
    }
}



#[cfg(test)]
mod test {
    //TODO test
    // - can not misset Content-Type
    // - can not set Content-Transfer-Encoding (done through ressource)
    // - above tests but wrt. set_headers/headers

    mod check_header {
        use headers::components::TransferEncoding;
        use headers::error::ComponentCreationError;
        use headers::{
            ContentType,
            ContentTransferEncoding,
        };
        use super::super::*;

        fn ct(s: &str) -> Result<<ContentType as Header>::Component, ComponentCreationError> {
            <&str as HeaderTryInto<_>>::try_into(s)
        }

        #[test]
        fn setting_non_multipart_headers_is_forbidden() {
            let comp = assert_ok!(ct("text/plain"));
            assert_err!(check_header::<ContentType>(&comp, false));
            let comp = assert_ok!(ct("multipart/plain"));
            assert_err!(check_header::<ContentType>(&comp, false));
        }

        #[test]
        fn setting_multi_on_multi_is_ok() {
            let comp = assert_ok!(ct("multipart/plain"));
            assert_ok!(check_header::<ContentType>(&comp, true));
        }

        #[test]
        fn setting_single_on_multi_is_err() {
            let comp = assert_ok!(ct("text/plain"));
            assert_err!(check_header::<ContentType>(&comp, true));
        }

        #[test]
        fn content_transfer_encoding_is_never_ok() {
            let comp = TransferEncoding::Base64;
            assert_err!(check_header::<ContentTransferEncoding>(&comp, true));
            assert_err!(check_header::<ContentTransferEncoding>(&comp, false));
        }
    }

    mod check_multiple_headers {
        use headers::components::TransferEncoding;
        use headers::{
            ContentType,
            ContentTransferEncoding,
        };
        use super::super::*;

        #[test]
        fn setting_non_multipart_headers_is_forbidden() {
            let headers = headers!{ ContentType: "text/plain" }.unwrap();
            assert_err!(check_multiple_headers(&headers, false));
            let headers = headers!{ ContentType: "multipart/plain" }.unwrap();
            assert_err!(check_multiple_headers(&headers, false));

        }

        #[test]
        fn setting_multi_on_multi_is_ok() {
            let headers = headers!{ ContentType: "multipart/plain" }.unwrap();
            assert_ok!(check_multiple_headers(&headers, true));
        }

        #[test]
        fn setting_single_on_multi_is_err() {
            let headers = headers!{ ContentType: "text/plain" }.unwrap();
            assert_err!(check_multiple_headers(&headers, true));
        }

        #[test]
        fn content_transfer_encoding_is_never_ok() {
            let headers = headers!{ ContentTransferEncoding: TransferEncoding::Base64 }.unwrap();
            assert_err!(check_multiple_headers(&headers, true));
            assert_err!(check_multiple_headers(&headers, false));
        }
    }
}