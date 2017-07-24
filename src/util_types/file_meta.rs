
use ascii::AsciiString;

use super::DateTime;

#[derive(Debug, Clone, Eq, PartialEq, Hash, Default)]
pub struct  FileMeta {
    // in rust std this is OsString, but we can not have it
    // os specific in any way, as it is send over internet,
    // originally this was Ascii, but has been extended
    // to support encoding
    // FEATURE_TODO(utf8_file_names): AsciiString => String
    pub file_name: Option<AsciiString>,
    pub creation_date: Option<DateTime>,
    pub modification_date: Option<DateTime>,
    pub read_date: Option<DateTime>,
    pub size: Option<usize>
}