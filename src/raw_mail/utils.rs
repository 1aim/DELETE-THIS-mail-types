use mime::Mime;
use mime::MULTIPART;

pub fn is_multipart_mime( mime: &Mime ) -> bool {
    mime.type_() == MULTIPART
}
