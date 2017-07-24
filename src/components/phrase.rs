use error::*;
use types::Vec1;
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

// while it is possible to store a single string,
// it is not future prove as some words can be given
// in a different encoding then the rest...
#[derive( Debug, Clone, Eq, PartialEq, Hash )]
pub enum Phrase {
    ItemBased( Vec1<Word> ),
    InputBased( Input )
}

impl Word {
    pub fn check_item_validity(item: Item) -> Result<()> {
        match item {
            Item::Ascii( ascii ) => {
                for ch in ascii.chars() {
                    if !is_atext( ch, MailType::Ascii ) {
                        bail!( "invalid atext (ascii) char: {}", ch );
                    }
                }
            },
            Item::Encoded( encoded ) => {
                let as_str = encoded.as_str();
                if !( is_encoded_word( as_str, EncodedWordContext::Phrase ) ||
                      is_quoted_word( as_str ){
                    bail!( "encoded item in context of phrase/word must be a encoded word" )
                }
            },
            Item::Utf8( international ) => {
                for ch in international.chars() {
                    if !is_atext( ch, MailType::Internationalized) {
                        bail!( "invalide atext (internationalized) char: {}", ch );
                    }
                }
            }
        }

        Ok( () )
    }

    pub fn new(item: InnerAsciiItem) -> Result<Self> {
        Self::check_item_validity( item )?;
        Ok( Word( None, item, None ) )
    }

    pub fn from_parts(
        left_padding: Option<CFWS>,
        item: InnerAsciiItem,
        right_padding: Option<CFWS>
    ) -> Result<Self> {

        Self::check_item_validity( item )?;
        Ok( Word( left_padding, item, right_padding ) )
    }

    pub fn pad_left( &mut self, padding: CFWS) {
        self.0 = Some( padding )
    }

    pub fn pad_right( &mut self, padding: CFWS) {
        self.2 = Some( padding )
    }

}

impl Phrase {

    pub fn from_words( words: Vec1<Word> ) -> Self {
        Phrase::ItemBased( words )
    }

    pub fn from_input( words: Input ) -> Self {
        Phrase::InputBased( words )
    }

}


pub fn mail_encode_word(
    word: &Word,
    encoder: &mut MailEncoder,
    ctx: EncodedWordContext
) -> Result<()> {

    if let Some( pad ) = word.0.as_ref() {
        pad.encode( encoder )?;
    }
    match *word.1 {
        Item::Ascii( ref ascii ) => {
            if ascii.as_str().starts_with( "=?" ) {
                encoder.write_encoded_word( ascii, EncodedWordContext::Phrase );
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
                encoder.write_encoded_word( utf8, EncodedWordContext::Phrase );
            }

        }
    }
    if let Some( pad ) = word.2.as_ref() {
        pad.encode( encoder )?;
    }
    Ok( () )
}



impl MailEncodable for Phrase  {

    //FEATURE_TODO(warn_on_bad_phrase): warn if the phrase contains chars it should not
    //  but can contain due to encoding, e.g. ascii CTL's
    fn encode(&self, encoder: &mut MailEncoder) -> Result<()> {
        use self::Phrase::*;

        match *self {
            ItemBased( ref words ) => {
                for word in words {
                    mail_encode_word(word, encoder, EncodedWordContext::Phrase )?;
                }
            },
            InputBased( ref input ) => {
                let mut last_ws = None;
                let mut scanning_ws_section = true;
                let mut section_start = 0;
                for (index, char) in input.char_indices() {
                    if is_ws( char ) {
                        if !scanning_ws_section {
                            //start next ws section
                            scanning_ws_section = true;

                            if let Some( last_ws ) = last_ws.take() {
                                encoder.write_str(
                                    //OPTIMIZE: use unsafe, it should only be '\t' or ' '
                                    AsciiStr::from_ascii( last_ws ).unwrap()
                                )
                            }
                            let word = input[ section_start..index ];
                            //writes to buffer unchecked if:
                            // word.chars().all( |ch| is_atext( ch, encoder.mail_type() ) )
                            if !encoder.try_write_atext( word ).is_ok() {
                                encoder.write_encoded_word( word )
                            }

                            section_start = index;
                        }
                    } else {
                        if scanning_ws_section {
                            //start next word section
                            scanning_ws_section = false;
                            //input starts with a word
                            if index == 0 { continue }

                            last_ws = Some( input[ section_start..index ])

                            section_start = index;
                        }
                    }
                }
            }

        }

    }
}


