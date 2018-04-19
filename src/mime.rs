use rand;
use rand::Rng;

use headers::components::MediaType;
use headers::error::ComponentCreationError;


/// write a random sequence of chars valide for and boundary to the output buffer
///
/// Note that it might be required to quote the boundary.
///
/// The boundary (excluding quotations) will start with `"=_"` which is neither
/// valid for base64 nor quoted-printable encoding.
///
/// The boundary will be 66 chars long, this is so that if a boundary parameter is
/// placed on it's own line it won't be more then 78 chars. (66 chars boundary,
/// + 2 chars quotation + 9 chars for 'boundary=' + 1 char because of `\r\n<WS>`
/// == 78 chars)
///
/// The remaining characters will be picked based one the grammar defined in rfc2046,
/// whichs relevant part is:
///
/// ```BNF
/// boundary := 0*69<bchars> bcharsnospace
/// bchars := bcharsnospace / " "
/// bcharsnospace := DIGIT / ALPHA / "'" / "(" / ")" /
///                  "+" / "_" / "," / "-" / "." /
///                  "/" / ":" / "=" / "?"
/// ```
///
pub fn create_random_boundary() -> String {


    //the maximal boundary with wich " boundary=\"...\"" fits into 78 chars line length limit
    const MULTIPART_BOUNDARY_LENGTH: usize = 66;
    // boundary chars based on rfc2046, excluding " "
    // (it can be used in any place _except_ the last)
    debug_assert!(1 <= MULTIPART_BOUNDARY_LENGTH && MULTIPART_BOUNDARY_LENGTH <= 70);
    static CHARS: &[char] = &[
        ' ',                               '\'',
        '(', ')',      '+', ',', '-', '.', '/',
        '0', '1', '2', '3', '4', '5', '6', '7',
        '8', '9', ':',           '=',      '?',
             'A', 'B', 'C', 'D', 'E', 'F', 'G',
        'H', 'I', 'J', 'K', 'L', 'M', 'N', 'O',
        'P', 'Q', 'R', 'S', 'T', 'U', 'V', 'W',
        'X', 'Y', 'Z',                     '_',
             'a', 'b', 'c', 'd', 'e', 'f', 'g',
        'h', 'i', 'j', 'k', 'l', 'm', 'n', 'o',
        'p', 'q', 'r', 's', 't', 'u', 'v', 'w',
        'x', 'y', 'z',
    ];

    // we add =_^ to the boundary, as =_^ is neither valid in base64 nor quoted-printable
    let mut out = String::with_capacity(MULTIPART_BOUNDARY_LENGTH);
    out.push_str("=_");
    let mut rng = rand::thread_rng();
    for _ in 2..(MULTIPART_BOUNDARY_LENGTH-1) {
        out.push( CHARS[ rng.gen_range( 0, CHARS.len() )] )
    }

    debug_assert!(CHARS[0] == ' ');
    let last_idx = rng.gen_range(1, CHARS.len());
    out.push(CHARS[last_idx]);
    out
}


pub fn gen_multipart_media_type<A>(subtype: A ) -> Result<MediaType, ComponentCreationError>
    where A: AsRef<str>
{
    let boundary = create_random_boundary();
    let media_type = MediaType::new_with_params("multipart", subtype.as_ref(), vec![
        ("boundary", &*boundary)
    ])?;
    Ok(media_type)
}



#[cfg(test)]
mod test {

    mod write_random_boundary_to {
        use super::super::*;

        #[test]
        fn boundary_is_not_quoted() {
            let out = create_random_boundary();
            assert!(!out.starts_with("\""));
            assert!(!out.ends_with("\""));
        }

        #[test]
        fn boundary_start_special() {
            let out = create_random_boundary();
            assert!(out.starts_with("=_"));
        }

        #[test]
        fn boundary_has_a_resonable_length() {
            let out = create_random_boundary();
            assert!(out.len() > 22 && out.len() < 100);
        }

        #[test]
        fn boundary_does_not_contain_space_or_slach_or_quotes() {
            // while it could contain them it's recommended not to do it
            let out = create_random_boundary();

            for ch in out[1..out.len()-1].chars() {
                assert!(ch as u32 >= 32);
                assert!(ch as u32 <= 126);
                assert_ne!(ch, '\t');
                assert_ne!(ch, '\\');
                assert_ne!(ch, '"');
            }

            assert_ne!(out.as_bytes()[out.len()-1], b' ');
        }
    }
}