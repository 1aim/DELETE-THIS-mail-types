use soft_ascii_string::SoftAsciiString;

use core::utils::uneraser_ref;
use core::error::{Result, ErrorKind};
use core::codec::EncodableInHeader;
use core::{ HeaderTryInto, Header, HeaderMap};
use mheaders::{
    ContentType,
    ContentTransferEncoding
};

use mheaders::components::MediaType;
//use mime::create_random_boundary;

use super::resource::Resource;
use super::{ MailPart, Mail };


pub struct Builder;

struct BuilderShared {
    headers: HeaderMap
}

pub struct SinglepartBuilder {
    inner: BuilderShared,
    body: Resource
}

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
    ) -> Result<usize>
        where H: Header,
              H::Component: EncodableInHeader
    {
        check_header::<H>(&hbody, is_multipart)?;
        self.headers.insert( header, hbody )
    }

    /// might already have added some headers even if it returns Err(...)
    fn headers( &mut self, headers: HeaderMap, is_multipart: bool ) -> Result<()> {
        //TODO CONSIDER:
        // it is not impossible to make this function "transactional" for HeaderMap
        // (it is impossible for TotalOrderMultiMap) by:
        // 1. implement pop on TotalOrderMultiMap
        // 2. store current len befor extending
        // 3. pop until the stored length is reached again
        check_multiple_headers( &headers, is_multipart )?;
        self.headers.extend( headers )?;
        Ok( () )
    }

    fn build( self, body: MailPart ) -> Result<Mail> {
        Ok( Mail {
            headers: self.headers,
            body: body,
        } )
    }
}

pub fn check_multiple_headers( headers: &HeaderMap , is_multipart: bool) -> Result<()> {
    if let Some( .. ) = headers.get_single(ContentTransferEncoding) {
        bail!( concat!(
            "setting content transfer encoding through a header is not supported,",
            "use Ressource::set_preferred_encoding on the body instead"
        ) );
    }
    //FIMXE[BUG] get->is_multipart seems wrong instead is_multipart->get?
    if let Some( mime ) = headers.get_single(ContentType) {
        if is_multipart {
            if !mime?.is_multipart() {
                return Err( ErrorKind::ContentTypeAndBodyIncompatible.into() )
            }
        } else {
            bail!( concat!(
                    "setting content type through a header for a single part body",
                    "is not supported use RessourceSpec::use_mime if you want to",
                    "override the content type"
                ) );
        }
    }
    Ok( () )
}

pub fn check_header<H>(
    hbody: &H::Component,
    is_multipart: bool
) -> Result<()>
    where H: Header,
          H::Component: EncodableInHeader
{
    match H::name().as_str() {
        "Content-Type" => {
            if is_multipart {
                let mime: &MediaType = uneraser_ref(hbody)
                    .ok_or_else( || "custom Content-Type headers are not supported" )?;
                if !mime.is_multipart() {
                    return Err( ErrorKind::ContentTypeAndBodyIncompatible.into() )
                }
            } else {
                bail!( concat!(
                    "setting content type through a header for a single part body",
                    "is not supported use RessourceSpec::use_mime if you want to",
                    "override the content type"
                ) );
            }

        },
        "Content-Transfer-Encoding" => {
            bail!( concat!(
                "setting content transfer encoding through a header is not supported,",
                "use Ressource::set_preferred_encoding on the body instead"
            ) );
        }
        _ => {}
    }
    Ok( () )
}

impl Builder {

    /// create a MultipartBuilder with the given media-type as content-type
    ///
    /// This function will always set the boundary parameter to a random
    /// generated boundary string. If the media type already had it
    /// boundary parameter it is overwritten.
    ///
    /// # Error
    ///
    /// if the media-type is not a `multipart/` media type an
    /// error is returned
    pub fn multipart(media_type: MediaType) -> Result<MultipartBuilder> {
        if !media_type.is_multipart() {
            return Err( ErrorKind::NotMultipartMime( media_type.into() ).into() );
        }

        //TODO implement set boundary
//        let mut media_type = media_type;
//        let boundary = create_random_boundary();
//        media_type.set_param(BOUNDARY, boundary);

        let res = MultipartBuilder {
            inner: BuilderShared::new(),
            hidden_text: None,
            bodies: Vec::new(),
        };

        //UNWRAP_SAFETY: it can only fail with illegal headers,
        // but this header can not be illegal
        Ok(res.header( ContentType, media_type ).unwrap())
    }

    pub fn singlepart( r: Resource ) -> SinglepartBuilder {
        SinglepartBuilder {
            inner: BuilderShared::new(),
            body: r,
        }
    }

}

impl SinglepartBuilder {

    pub fn header<H, C>(
        &mut self,
        header: H,
        hbody: C
    ) -> Result<usize>
        where H: Header,
              H::Component: EncodableInHeader,
              C: HeaderTryInto<H::Component>
    {
        let comp = hbody.try_into()?;
        self.inner.header( header, comp, false )
    }

    pub fn headers( mut self, headers: HeaderMap ) -> Result<Self> {
        self.inner.headers( headers, false )?;
        Ok( self )
    }

    pub fn build( self ) -> Result<Mail> {

        self.inner.build( MailPart::SingleBody { body: self.body } )
    }
}

impl MultipartBuilder {


    ///
    /// # Error
    ///
    /// A error is returned if the header is incompatible with this builder,
    /// i.e. if a ContentType header is set with a non-multipart content type
    pub fn header<H, C>(
        mut self,
        header: H,
        hbody: C
    ) -> Result<Self>
        where H: Header,
              H::Component: EncodableInHeader,
              C: HeaderTryInto<H::Component>
    {
        let comp = hbody.try_into()?;
        self.inner.header( header, comp, true )?;
        Ok( self )
    }

    pub fn headers( mut self, headers: HeaderMap ) -> Result<Self> {
        self.inner.headers( headers, true )?;
        Ok( self )
    }

    pub fn body( mut self, body: Mail ) -> Result<Self> {
        self.bodies.push( body );
        Ok( self )
    }

    pub fn build( self ) -> Result<Mail> {
        if self.bodies.len() == 0 {
            Err( ErrorKind::NeedAtLastOneBodyInMultipartMail.into() )
        } else {
            self.inner.build( MailPart::MultipleBodies {
                bodies: self.bodies,
                hidden_text: self.hidden_text.unwrap_or( SoftAsciiString::new() ),
            } )
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
        use mheaders::components::TransferEncoding;
        use mheaders::{
            ContentType,
            ContentTransferEncoding,
        };
        use super::super::*;

        fn ct(s: &str) -> Result<<ContentType as Header>::Component> {
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
        use mheaders::components::TransferEncoding;
        use mheaders::{
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