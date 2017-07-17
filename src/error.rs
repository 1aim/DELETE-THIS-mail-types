use mime::Mime;

error_chain! {


    errors {
        /// Certain components might not be encodable under some circumstances.
        /// E.g. they might have non-ascii values and are not encodable into ascii
        ///
        /// a example for this would be a non ascii `local-part` of `addr-spec`
        /// (i.e. the part of a email address befor the `@`)
        NonEncodableComponents( component: &'static str, data: String ) {
            description( "given information can not be encoded into ascii" )
            display( "can not encode the {} component with value {:?}", component, data )
        }

        TriedWriting8BitBytesInto7BitData {
            description(
                "the program tried to write a non ascii string while smtputf8 was not supported" )
        }

        AtLastOneElementIsRequired {
            description( concat!( "for the operation a list with at last one element",
                                  " is required but and empty list was given" ) )
        }

        InvalidHeaderName(name: String) {
            description( "given header name is not valid" )
            display( "{:?} is not a valid header name", name )
        }

        NotMultipartMime( mime: Mime ) {
            description( "expected a multipart mime for a multi part body" )
            display( _self ) -> ( "{}, got: {}", _self.description(), mime )
        }

        MultipartBoundaryMissing {
            description( "multipart boundary is missing" )
        }

        NotSinglepartMime( mime: Mime ) {
            description( "expected a non-multipart mime for a non-multipart body" )
            display( _self ) -> ( "{}, got: {}", _self.description(), mime )
        }

        NeedPlainAndOrHtmlMailBody {

        }
    }
}
