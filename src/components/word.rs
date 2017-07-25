use error::*;
use codec::{ MailEncodable, MailEncoder };
use ascii::AsciiStr;

use super::utils::item::{ Input, Item };

use char_validators::{
    is_ws, is_atext, MailType
};

use char_validators::encoded_word::{
    is_encoded_word,
    EncodedWordContext
};

use super::CFWS;

#[derive( Debug, Clone, Eq, PartialEq, Hash )]
pub struct Word(Option<CFWS>, Item, Option<CFWS> );


impl Word {
    pub fn check_item_validity(item: &Item, allow_encoded_word: bool) -> Result<()> {
        match *item {
            Item::Ascii( ref ascii ) => {
                for ch in ascii.chars() {
                    if !is_atext( ch, MailType::Ascii ) {
                        bail!( "invalid atext (ascii) char: {}", ch );
                    }
                }
            },
            Item::Encoded( ref encoded ) => {
                let as_str = encoded.as_str();
                if !( (allow_encoded_word && is_encoded_word( as_str, EncodedWordContext::Phrase ) )
                      || is_quoted_word( as_str ) )
                {
                    bail!( "encoded item in context of phrase/word must be a encoded word" )
                }
            },
            Item::Utf8( ref international ) => {
                for ch in international.chars() {
                    if !is_atext( ch, MailType::Internationalized) {
                        bail!( "invalide atext (internationalized) char: {}", ch );
                    }
                }
            }
        }

        Ok( () )
    }

    pub fn new( item: Item, allow_encoded_word: bool ) -> Result<Self> {
        Self::check_item_validity( &item, allow_encoded_word )?;
        Ok( Word( None, item, None ) )
    }

    pub fn from_parts(
        left_padding: Option<CFWS>,
        item: Item,
        right_padding: Option<CFWS>,
        allow_encoded_word: bool
    ) -> Result<Self> {

        Self::check_item_validity( &item, allow_encoded_word )?;
        Ok( Word( left_padding, item, right_padding ) )
    }

    pub fn pad_left( &mut self, padding: CFWS) {
        self.0 = Some( padding )
    }

    pub fn pad_right( &mut self, padding: CFWS) {
        self.2 = Some( padding )
    }

}

/// != encoded-word, through it might create an encoded-word
pub fn do_encode_word<E: MailEncoder>(
    word: &Word,
    encoder: &mut E,
    ew_ctx: Option<EncodedWordContext>,
) -> Result<()> {

    if let Some( pad ) = word.0.as_ref() {
        pad.encode( encoder )?;
    }
    match *word.1 {
        Item::Ascii( ref ascii ) => {
            // if it is no encoded word, but could be one and starts like one
            // then we better encode it to make sure it's not interpreted as a
            // encoded word
            if ew_ctx.is_some() && ascii.as_str().starts_with( "=?" ) {
                encoder.write_encoded_word( ascii, ew_ctx.unwrap() );
            } else {
                encoder.write_str( ascii );
            }
        },
        Item::Encoded( ref enc ) => {
            //word+Item::Encoded, already checked if "encoded" == "encoded word"
            // OR == "quoted string" in both cases we can just write it
            encoder.write_str( &*enc )
        },
        Item::Utf8( ref utf8 ) => {
            if encoder.mail_type() == MailType::Internationalized {
                encoder.write_str( utf8 );
            } else {
                if let Some( ew_ctx ) = ew_ctx {
                    encoder.write_encoded_word( utf8, ew_ctx );
                } else {
                    bail!( "can not encode utf8 for this word, as encoded-words are not allowed" );
                }
            }

        }
    }
    if let Some( pad ) = word.2.as_ref() {
        pad.encode( encoder )?;
    }
    Ok( () )
}