use failure::Fail;

use common::EncodingError;
use headers::{
    HeaderTypeError, ComponentCreationError,
    HeaderInsertionError, HeaderValidationError
}

// errors from loading a Resource (which includes encoding it's body)
//                /  NotFound       | Iri (no Backtrace neede)     \ MailError::ResourceLoading
// ResourceError <   LoadingFailed  | chain Error                  /
//                \  EncodingFailed | EncodingError (Backtrace!)   > MailError::Encoding
//

#[derive(Debug, Fail)]
pub enum ResourceError {
    #[fail(display = "{}", _0)]
    Loading(ResourceLoadingError)

    #[fail(display = "{}", _0)]
    Encoding(EncodingError)
}

impl From<EncodingError> for ResourceError {
    fn from(err: EncodingError) -> Self {
        ResourceError::Encoding(EncodingError)
    }
}

impl From<ResourceLoadingError> for ResourceError {
    fn from(err: ResourceLoadingError) -> Self {
        ResourceError::Loading(err)
    }
}


#[derive(Copy, Clone, Debug, Fail, PartialEq, Eq, Hash)]
pub enum ResourceLoadingErrorKind {
    #[fail(display = "resource not found")]
    NotFound,

    #[fail(display = "loading shared resource already failed before")]
    SharedResourcePoisoned,

    #[fail(display = "loading failed")]
    LoadingFailed
}

//TODO I just noticed that all error variants would nice to
// 1. have the IRI
// 2. have a Backtrace or Error
//
// => so make it ResourceLoadingKind + ResourceLoadingError { Context<Kind>, IRI }
#[derive(Debug)]
pub struct ResourceLoadingError {
    inner: Context<ResourceLoadingErrorKind>,
    iri: Option<Iri>
}

impl Fail for ResourceLoadingError {
    fn cause(&self) -> Option<&Fail> {
        self.inner.cause()
    }

    fn backtrace(&self) -> Option<&Backtrace> {
        self.inner.backtrace()
    }
}

impl ResourceLoadingError {

    pub fn kind(&self) -> ResourceLoadingErrorKind {
        *self.inner.get_context()
    }

    pub fn source_iri(&self) -> Option<&Iri> {
        self.iri.as_ref()
    }

    pub fn with_source_iri_or_else<F>(self, func: F) -> Self
        where F: FnOnce() -> Option<Iri>
    {
        if self.iri.is_none() {
            self.iri = func();
        }
        self
    }
}

impl From<ResourceLoadingErrorKind> for ResourceLoadingError {
    fn from(err: ResourceLoadingErrorKind) -> Self {
        ResourceLoadingError::from((None, err))
    }
}

impl From<Context<ResourceLoadingErrorKind>> for ResourceLoadingError {
    fn from(inner: Context<ResourceLoadingErrorKind>) -> Self {
        ResourceLoadingError::from((None, inner))
    }
}

impl From<(Iri, ResourceLoadingErrorKind)> for ResourceLoadingError {
    fn from((iri, error_kind): (Iri, ResourceLoadingErrorKind)) -> Self {
        ResourceLoadingError::from((Some(iri), error_kind))
    }
}

impl From<(Iri, Context<ResourceLoadingErrorKind>)> for ResourceLoadingError {
    fn from((iri, inner): (Iri, Context<ResourceLoadingErrorKind>)) -> Self {
        ResourceLoadingError::from((Some(iri), inner))
    }
}

impl From<(Option<Iri>, ResourceLoadingErrorKind)> for ResourceLoadingError {
    fn from((iri, error_kind): (Option<Iri>, ResourceLoadingErrorKind)) -> Self {
        ResourceLoadingError::from((iri, Context::new(error_kind)))
    }
}

impl From<(Option<Iri>, Context<ResourceLoadingErrorKind>)> for ResourceLoadingError {
    fn from((iri, inner): (Option<Iri>, Context<ResourceLoadingErrorKind>)) -> Self {
        ResourceLoadingError {
            inner, iri
        }
    }
}




#[derive(Copy, Clone, Debug, Fail, PartialEq, Eq, Hash)]
pub enum OtherBuilderErrorKind {
    #[fail(display = "inserting Content-Type manually is not allowed")]
    InsertingContentTypeHeader,

    #[fail(display = "inserting Content-Transfer-Encoding manually is not allowed")]
    InsertingContentTransferEncodingHeader,

    #[fail(display = "inserting a header changing if a body is single/multipart is not allowed")]
    SingleMultipartMixup,

    #[fail(display = "inserting Content-Type for singlepart body is not allowed")]
    InsertSinglepartContentTypeHeader,
}

#[derive(Debug, Fail)]
pub enum BuilderError {
    #[fail(display = "{}", _0)]
    Type(HeaderTypeError),

    #[fail(display = "{}", _0)]
    Component(ComponentCreationError),

    #[fail(display = "{}", _0)]
    Other(Context<OtherBuilderErrorKind>)
}

impl From<OtherBuilderErrorKind> for BuilderError {
    fn from(err: OtherBuilderErrorKind) -> Self {
        BuilderError::Other(Context::new(err))
    }
}

impl From<Context<OtherBuilderErrorKind>> for BuilderError {
    fn from(err: Context<OtherBuilderErrorKind>) -> Self {
        BuilderError::Other(err)
    }
}

impl From<HeaderTypeError> for BuilderError {
    fn from(err: HeaderTypeError) -> Self {
        BuilderError::Type(err)
    }
}

impl From<ComponentCreationError> for BuilderError {
    fn from(err: ComponentCreationError) -> Self {
        BuilderError::Component(err)
    }
}

impl From<HeaderInsertionError> for BuilderError {
    fn from(err: HeaderInsertionError) -> Self {
        use self::HeaderInsertionError::*;
        match err {
            Type(err) => BuilderError::Type(err),
            Component(err) => BuilderError::Component(err)
        }
    }
}


#[derive(Debug, Fail)]
pub enum MailError {
    #[fail(display = "{}", _0)]
    Encoding(EncodingError),

    #[fail(display = "{}", _0)]
    Creation(BuilderError),

    #[fail(display = "{}", _0)]
    Validation(HeaderValidationError),

    #[fail(display = "{}", _0)]
    ResourceLoading(ResourceLoadingError)
}


impl From<EncodingError> for MailError {
    fn from(err: EncodingError) -> Self {
        MailError::Encoding(err)
    }
}

impl From<BuilderError> for MailError {
    fn from(err: HeaderInsertionError) -> Self {
        MailError::Creation(err)
    }
}

impl From<HeaderValidationError> for MailError {
    fn from(err: HeaderValidationError) -> Self {
        MailError::Validation(err)
    }
}

impl From<ResourceLoadingError> for MailError {
    fn from(err: ResourceLoadingError) -> Self {
        MailError::ResourceLoading(err)
    }
}

impl From<ResourceError> for MailError {
    fn from(err: ResourceError) -> Self {
        match err {
            ResourceError::Loading(err) => MailError::Loading(err),
            ResourceError::Encoding(err) => MailError::Encoding(err)
        }
    }
}