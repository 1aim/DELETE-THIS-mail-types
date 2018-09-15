use std::path::{ PathBuf, Path };
use std::fs::File;
use std::io::{self, Read};
use std::env;
use std::marker::PhantomData;

use failure::Fail;
use futures::{ future, IntoFuture};

use headers::header_components::{
    MediaType,
    FileMeta
};

use ::IRI;
use ::utils::{ConstSwitch, Enabled, Disabled};
use ::error::{ResourceLoadingError, ResourceLoadingErrorKind};
use ::file_buffer::FileBuffer;
use ::context::{ResourceLoaderComponent, OffloaderComponent, Source, LoadResourceFuture};

// have a scheme ignoring variant for Mux as the scheme is preset
// allow a setup with different scheme path/file etc. the behavior stays the same!
// do not handle sandboxing/security as such do not handle "file" only "path" ~use open_at if available?~

//TODO more doc
/// By setting SchemeValidation to Disabled the FsResourceLoader can be used to simple
/// load a resource from a file based on a scheme tail as path independent of the rest,
/// so e.g. it it is used in a `Mux` which selects a `ResourceLoader` impl based on a scheme
/// the scheme would not be double validated.
#[derive( Debug, Clone, PartialEq, Default )]
pub struct FsResourceLoader<
    SchemeValidation: ConstSwitch = Enabled,
    // we do not want to fix newlines for embeddings/attachments they get transfer encoded base64
    // just for templates this makes sense
    FixNewlines: ConstSwitch = Disabled,
> {
    root: PathBuf,
    scheme: &'static str,
    _marker: PhantomData<(SchemeValidation, FixNewlines)>
}

impl<SVSw, NLSw> FsResourceLoader<SVSw, NLSw>
    where SVSw: ConstSwitch, NLSw: ConstSwitch
{

    const DEFAULT_SCHEME: &'static str = "path";

    /// create a new file system based FileLoader, which will  "just" standard _blocking_ IO
    /// to read a file from the file system into a buffer
    pub fn new<P: Into<PathBuf>>( root: P ) -> Self {
        Self::new_with_scheme(root.into(), Self::DEFAULT_SCHEME)
    }

    pub fn new_with_scheme<P: Into<PathBuf>>( root: P, scheme: &'static str ) -> Self {
        FsResourceLoader { root: root.into(), scheme, _marker: PhantomData}
    }

    pub fn with_cwd_root() -> Result<Self, io::Error> {
        let cwd = env::current_dir()?;
        Ok(Self::new(cwd))
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn scheme(&self) -> &'static str {
        self.scheme
    }

    pub fn does_validate_scheme(&self) -> bool {
        SVSw::ENABLED
    }

    pub fn iri_has_compatible_scheme(&self, iri: &IRI) -> bool {
        iri.scheme() == self.scheme
    }
}


impl<ValidateScheme, FixNewlines> ResourceLoaderComponent
    for FsResourceLoader<ValidateScheme, FixNewlines>
    where ValidateScheme: ConstSwitch, FixNewlines: ConstSwitch
{

    fn load_resource<O>( &self, source: &Source, offload: &O) -> LoadResourceFuture
        where O: OffloaderComponent
    {
        if ValidateScheme::ENABLED && !self.iri_has_compatible_scheme(&source.iri) {
            let err = ResourceLoadingError
                ::from(ResourceLoadingErrorKind::NotFound)
                .with_source_iri_or_else(|| Some(source.iri.clone()));

            return Box::new(Err(err).into_future());
        }

        let path = self.root().join(path_from_tail(&source.iri));
        let media_type = source.use_media_type.clone();
        let name = source.use_name.clone();

        offload.offload(
            future::lazy(move || {
                load_file_buffer::<FixNewlines>(path, media_type, name)
            })
        )
    }
}


//TODO add a PostProcess hook which can be any combination of
// FixNewline, SniffMediaType and custom postprocessing
// now this has new responsibilities
// 2. get and create File Meta
// 3. if source.media_type.is_none() do cautious mime sniffing
fn load_file_buffer<
    FixNewlines: ConstSwitch
>(path: PathBuf, media_type: Option<MediaType>, name: Option<String>)
    -> Result<FileBuffer, ResourceLoadingError>
{


    let mut fd = File::open(&path)
        .map_err(|err| {
            if err.kind() == io::ErrorKind::NotFound {
                err.context(ResourceLoadingErrorKind::NotFound)
            } else {
                err.context(ResourceLoadingErrorKind::LoadingFailed)
            }
        })?;

    let mut file_meta = file_meta_from_metadata(fd.metadata()?);
    if let Some(name) = name {
        file_meta.file_name = Some(name)
    } else {
        file_meta.file_name = path.file_name()
            .map(|name| name.to_string_lossy().into_owned())
    }

    let mut buffer = Vec::new();
    fd.read_to_end(&mut buffer)?;

    if FixNewlines::ENABLED {
        buffer = fix_newlines(buffer);
    }

    let media_type =
        if let Some(mt) = media_type {
            mt
        } else {
            sniff_media_type(&buffer)?
        };

    Ok(FileBuffer::with_file_meta(media_type, buffer, file_meta))

}

fn sniff_media_type(_buffer: &[u8]) -> Result<MediaType, ResourceLoadingError> {
    //TODO replace current stub impl with conservative_sniffing and move it to mail
    unimplemented!();
}

fn fix_newlines(buffer: Vec<u8>) -> Vec<u8> {
    //TODO replace current stub impl with fix_newlines impl from mail-template
    // and move fix_newlines to core
    let mut hit_cr = false;
    for bch in buffer.iter() {
        match *bch {
            b'\r' => if hit_cr { unimplemented!() } else { hit_cr=true; },
            b'\n' => if !hit_cr { unimplemented!() } else { hit_cr=false; },
            _ => if hit_cr { unimplemented!() } else {}
        }
    }
    if hit_cr {
        unimplemented!()
    }
    buffer
}

use std::fs::Metadata;
//TODO implement From<MetaDate> for FileMeta instead of this
fn file_meta_from_metadata(meta: Metadata) -> FileMeta {
    FileMeta {
        file_name: None,
        creation_date: meta.created().ok().map(From::from),
        modification_date: meta.modified().ok().map(From::from),
        read_date: meta.accessed().ok().map(From::from),
        //TODO make FileMeta.size a u64
        size: get_file_size(&meta).map(|x|x as usize),
    }
}

fn get_file_size(meta: &Metadata) -> Option<u64> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        return Some(meta.size());
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        return Some(meta.file_size());
    }
    #[allow(unreachable_code)]
    None
}

fn path_from_tail(path_iri: &IRI) -> &Path {
    let tail = path_iri.tail();
    let path = if tail.starts_with("///") {
        &tail[2..]
    } else {
        &tail
    };
    Path::new(path)
}


