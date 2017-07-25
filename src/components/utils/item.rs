
use std::rc::Rc;
use std::ops::{ Deref, DerefMut };
use std::result::{ Result as StdResult };

use owning_ref::OwningRef;
use ascii::{ AsciiString, FromAsciiError };

//FEATURE_TODO(non_utf8_input): use (Vec<u8>, Encoding) instead of String in Input
//  but keep String in item, as there non utf8 input is not allowed

/// a Input is similar to Item a container data container used in different
/// context's with different restrictions, but different to an Item it
/// might contain characters which require encoding (e.g. encoded words)
/// to represent them
#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub enum Input {
    Owned(String),
    Shared(OwningRef<Rc<String>, str>)
}

pub enum Item {
    Ascii( InnerAsciiItem ),
    Encoded( InnerAsciiItem ),
    Utf8( InnerUtf8Item ),
    //Other( InnerOtherItem )
}

pub enum SimpleItem {
    /// specifies that the Item is valid Ascii, nothing more
    Ascii( InnerAsciiItem ),
    /// specifies that the Item is valid Utf8, nothing more
    Utf8( InnerUtf8Item )
}

impl Input {

    pub fn into_ascii_item( self ) -> StdResult<InnerAsciiItem, FromAsciiError<String>> {
        Ok( match self {
            Input::Owned( string )
                => InnerAsciiItem::Owned( AsciiString::from_ascii( string )? ),
            Input::Shared( shared )
                => InnerAsciiItem::Owned(
                    AsciiString::from_ascii( String::from( &*shared ) )? )
        } )
    }

    pub unsafe fn into_ascii_item_unchecked( self ) -> InnerAsciiItem {
        match self {
            Input::Owned( string )
                => InnerAsciiItem::Owned( AsciiString::from_ascii_unchecked( string ) ),
            Input::Shared( shared )
                => InnerAsciiItem::Owned(
                    AsciiString::from_ascii_unchecked( String::from( &*shared ) ) )
        }
    }

    pub fn into_utf8_item( self ) -> InnerUtf8Item {
        match self {
            Input::Owned( string ) => InnerUtf8Item::Owned( string ),
            Input::Shared( orwf ) => InnerUtf8Item::Shared( orwf )
        }
    }
}
impl From<String> for Input {
    fn from( s: String ) -> Self {
        Input::Owned( s )
    }
}

impl Deref for Input {
    type Target = str;

    fn deref( &self ) -> &str {
        use self::Input::*;
        match *self {
            Owned( ref string ) => &*string,
            Shared( ref owning_ref ) => &*owning_ref
        }
    }
}


macro_rules! inner_impl {
    ($name:ty, $owned_form:ty, $borrowed_form:ty) => (

        /// a InnerItem is something potential appearing in Mail, e.g. an encoded word, an
        /// atom or a email address, but not some content which has to be represented
        /// as an encoded word, as such String is a suite representation,
        #[derive(Debug, Clone, Hash, Eq, PartialEq)]
        pub enum $name {
            Owned($owned_form),
            Shared(OwningRef<Rc<$owned_form>, $borrowed_form>)
        }

        impl $name {
            pub fn new<S: Into<$owned_form>>( data: S ) -> $name {
                $name::Owned( data.into() )
            }
        }

        impl<S> From<S> for $name where S: Into<$owned_form> {
            fn from( data: S ) -> Self {
                Self::new( data )
            }
        }

        impl Deref for $name {
            type Target = $borrowed_form;

            fn deref( &self ) -> &$borrowed_form{
                match *self {
                    $name::Owned( ref string ) => &*string,
                    $name::Shared( ref owning_ref ) => &*owning_ref
                }
            }
        }

    )
}

inner_impl!{ InnerAsciiItem, AsciiString, AsciiStr }
inner_impl!{ InnerUtf8Item, String, str }
//inner_impl!{ InnerOtherItem, OtherString, OtherStr }