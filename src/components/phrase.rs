use error::*;
use types::Vec1;
use codec::{MailEncodable, MailEncoder };
use ascii::AsciiStr;

use super::utils::item::{ Input, Item };
use super::word::{Word, do_encode_word};

use char_validators::is_ws;
use char_validators::encoded_word::EncodedWordContext;

#[derive( Debug, Clone, Eq, PartialEq, Hash )]
pub struct PhraseWord( Word );

impl PhraseWord {
    pub fn new( item: InnerAsciiItem ) -> Result<Self> {
        Ok( PhraseWord( Word::new( item, true )? ) )
    }

    pub fn from_parts(
        left_padding: Option<CFWS>,
        item: InnerAsciiItem,
        right_padding: Option<CFWS>,
    ) -> Result<Self> {
        Ok( PhraseWord( Word::from_parts( left_padding, item, right_padding, true )? ) )
    }

}

deref0!{ +mut PhraseWord, Word }

// while it is possible to store a single string,
// it is not future prove as some words can be given
// in a different encoding then the rest...
#[derive( Debug, Clone, Eq, PartialEq, Hash )]
pub enum Phrase {
    ItemBased( Vec1<Word> ),
    InputBased( Input )
}


impl Phrase {

    pub fn from_words( words: Vec1<Word> ) -> Self {
        Phrase::ItemBased( words )
    }

    pub fn from_input( words: Input ) -> Self {
        Phrase::InputBased( words )
    }

}



impl MailEncodable for Phrase  {

    //FEATURE_TODO(warn_on_bad_phrase): warn if the phrase contains chars it should not
    //  but can contain due to encoding, e.g. ascii CTL's
    fn encode<E>( &self, encoder:  &mut E ) -> Result<()> where E: MailEncoder {
        use self::Phrase::*;

        match *self {
            ItemBased( ref words ) => {
                for word in words {
                    do_encode_word( &*word, encoder, Some( EncodedWordContext::Phrase ) )?;
                }
            },
            //TODO handle input not containing a valid phrase (only ws's)
            //TODO escape comments
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


