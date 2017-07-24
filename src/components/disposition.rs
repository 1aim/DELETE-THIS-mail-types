//FIXME use Fnv?
use std::collections::HashMap;
use std::ops::Deref;

use ascii::{ AsciiChar, AsciiStr };

//this will be moved to some where where the import of it is ok
use super::components::behaviour::utils::is_token_char;

use util_types::FileMeta;
use error::*;
use codec::{ MailEncodable, MailEncoder };



#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct Disposition {
    kind: DispositionKind,
    file_meta: DispositionParameters(FileMeta)
}

#[derive(Debug, Clone, Eq, PartialEq, Hash, Default)]
struct DispositionParameters(FileMeta);

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum DispositionKind {
    Inline, Attachment
}

impl Disposition {

    pub fn inline() -> Self {
        Disposition::new( DispositionKind::Inline, DispositionParameters::default() )
    }

    pub fn attachment() -> Self {
        Disposition::new( DispositionKind::Attachment, DispositionParameters::default() )
    }

    pub fn new( kind: DispositionKind, file_meta: FileMeta ) {
        Disposition { kind, file_meta: DispositionParameters( file_meta ) }
    }

    pub fn kind( &self ) -> DispositionKind {
        self.kind
    }

    pub fn file_meta(&self ) -> &FileMeta {
        &self.file_meta
    }

    pub fn file_meta_mut( &self, ) -> &mut FileMeta {
        &mut self.file_meta
    }

}

macro_rules! encode_disposition_param {
    ( do $($ch:ident)* | $value:expr | $inner:ident => $code:block ) => ({
        if let Some( ref $inner ) = $value {
            encoder.write_char( AsciiChar::Semicolon );
            encoder.write_str( ascii_str!{ $($ch)* } );
            encoder.write_char( AsciiChar::Equal );
            $code
        }
    });

    ( $( $tp:tt $($ch:ident)* | $value:expr; )* ) => ({
        $(encode_disposition_param ! ( $ tp $ ( $ ch) * | $ value ))*
    });

    ( STR $($ch:ident)* | $value:expr ) => (
        encode_disposition_param!( do $($ch)* | $value | filename => {
            encode_file_name( &**file_name, encoder )?;
        })
    );
    ( DATE $($ch:ident)* | $value:expr ) => (
        encode_disposition_param!( do $($ch)* | $value | date => {
            encoder.write_char( AsciiChar::Quotation );
            date.encode( encoder )?;
            encoder.write_char( AsciiChar::Quotation );
        })
    );
    ( USIZE $(ch:ident)* | $value:expr ) => (
        encode_disposition_param!( do $($ch)* | $value | val => {
            let val: usize = val;
            encoder.write_str( AsciiStr::from_ascii_unchecked( val.to_string() ) );
        })
    );
}

//TODO provide a gnneral way for encoding header parameter ...
//  which follow the scheme: <mainvalue> *(";" <key>"="<value> )
//  this are: ContentType and ContentDisposition for now
impl MailEncodable for DispositionParameters {
    fn encode( &self, encoder: &mut MailEncoder ) -> Result<()> {
        encode_disposition_param! {
            STR f i l e n a m e | self.file_name;
            DATE c r e a t i o n Minus d a t e | self.creation_date;
            DATE m o d i f i c a t i o n Minus d a t e | self.modification_date;
            DATE r e a d Minus d a t e | self.read_date;
            USIZE s i z e | self.size;
        }
        Ok( () )
    }
}


fn encode_file_name( file_name: &AsciiStr, encoder: &mut MailEncoder) -> Result<()> {
    for char in file_name {
        if !is_token_char( char ) {
            bail!(
                "handling non token file names in ContentDisposition is currently not supported" );
        }
    }
    encoder.write_str( file_name );
    Ok( () )
}


impl Deref for DispositionParameters {
    type Target = FileMeta;

    fn deref( &self ) -> &FileMeta {
        &self.0
    }
}
