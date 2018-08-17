use std::str::FromStr;

//TODO consider adding a str_context
#[derive(Copy, Clone, Debug, Fail)]
#[fail(display = "invalid syntax for iri/uri scheme")]
pub struct InvalidIRIScheme;

/// A minimal IRI (International Resource Identifier) implementation which just
/// parses the scheme but no scheme specific part (and neither fragments wrt.
/// those definitions in which fragments are not scheme specific parts).
///
/// **This implementation does not perform any form of normalization or other
/// IRI specific aspects, it's basically just a String split into two parts.**
///
/// Additionally this implementations requires all URI to be valid utf8.
///
/// # Example
///
/// ```
/// # use mail_types::IRI;
/// let uri = IRI::new("file:/random/logo.png").unwrap();
/// assert_eq!(uri.scheme(), "file");
/// assert_eq!(uri.tail(), "/random/logo.png");
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Ord, PartialOrd, Hash)]
pub struct IRI {
    iri: String,
    scheme_end_idx: usize
}

impl IRI {

    /// create a new IRI from a scheme part and a tail part
    pub fn from_parts(scheme: &str, tail: &str) -> Result<Self, InvalidIRIScheme> {
        Self::validate_scheme(scheme)?;
        let scheme_len = scheme.len();
        let mut buffer = String::with_capacity(scheme_len + 1 + tail.len());
        buffer.push_str(scheme);
        buffer.push(':');
        buffer.push_str(tail);
        Ok(IRI {
            iri: buffer,
            scheme_end_idx: scheme_len
        })
    }

    /// crates a new a IRI
    ///
    /// 1. this determines the first occurrence of `:` to split the input into scheme and tail
    /// 2. it validates that the scheme name is [RFC 3986](https://tools.ietf.org/html/rfc3986)
    ///    compatible, i.e. is ascii, starting with a letter followed by alpha numeric characters
    ///    (or `"+"`,`"-"`,`"."`).
    /// 3. converts the scheme part to lower case
    pub fn new<I>(iri: I) -> Result<Self, InvalidIRIScheme>
        where I: Into<String>
    {
        let mut buffer = iri.into();
        let split_pos = buffer.bytes().position(|b| b == b':')
            //TODO error type
            .ok_or_else(|| InvalidIRIScheme)?;
        {
            let scheme = &mut buffer[..split_pos];
            {
                Self::validate_scheme(scheme)?;
            }

            scheme.make_ascii_lowercase();
        }

        Ok(IRI {
            iri: buffer,
            scheme_end_idx: split_pos
        })
    }

    fn validate_scheme(scheme: &str) -> Result<(), InvalidIRIScheme> {
        let mut iter = scheme.bytes();
        let valid = iter.next()
            .map(|bch| bch.is_ascii_alphabetic()).unwrap_or(false)
            && iter.all(|bch|
                bch.is_ascii_alphanumeric() || bch == b'+' || bch == b'-' || bch == b'.');

        if !valid {
            return Err(InvalidIRIScheme);
        }
        Ok(())
    }

    /// the scheme part of the uri excluding the `:` seperator
    ///
    /// # Example
    ///
    /// ```
    /// # use mail_types::IRI;
    /// let uri = IRI::new("file:///opt/share/logo.png").unwrap();
    /// assert_eq!(uri.scheme(), "file");
    /// ```
    pub fn scheme(&self) -> &str {
        &self.iri[..self.scheme_end_idx]
    }

    /// the scheme specific part of the uri
    ///
    /// # Example
    ///
    /// ```
    /// # use mail_types::IRI;
    /// let uri = IRI::new("file:///opt/share/logo.png").unwrap();
    /// assert_eq!(uri.scheme(), "file");
    /// ```
    pub fn tail(&self) -> &str {
        &self.iri[self.scheme_end_idx+1..]
    }

    /// returns the underlying string representation
    ///
    /// Note that it does not implement Display even through
    /// it implements `as_str` and `Into<String>` as displaying
    /// a IRI is more complex then just displaying a string (mainly due to
    /// bidirectional IRI's).
    pub fn as_str(&self) -> &str {
        &self.iri
    }
}

impl FromStr for IRI {
    type Err = InvalidIRIScheme;

    fn from_str(inp: &str) -> Result<Self, Self::Err> {
        IRI::new(inp)
    }
}

impl Into<String> for IRI {
    fn into(self) -> String {
        self.iri
    }
}

#[cfg(test)]
mod test {
    use super::IRI;

    #[test]
    fn split_correctly_excluding_colon() {
        let uri = IRI::new("scheme:other:parts/yeha?z=r#frak").unwrap();
        assert_eq!(uri.scheme(), "scheme");
        assert_eq!(uri.tail(), "other:parts/yeha?z=r#frak");
        assert_eq!(uri.as_str(), "scheme:other:parts/yeha?z=r#frak");
    }

    #[test]
    fn scheme_is_lowercase() {
        let uri = IRI::new("FILE:///opt/share/logo.png").unwrap();
        assert_eq!(uri.scheme(), "file");
        assert_eq!(uri.as_str(), "file:///opt/share/logo.png");
    }

    #[test]
    fn scheme_name_has_to_be_valid() {
        // empty scheme
        assert!(IRI::new(":ups").is_err());
        // starting with numeric scheme
        assert!(IRI::new("1aim.path:/logo").is_err());
        // schme with invalid chars
        assert!(IRI::new("g ap:ups").is_err());
        assert!(IRI::new("s{trang}e:ups").is_err());

        // some strange but valid names
        assert!(IRI::new("c++:is valid").is_ok());
        assert!(IRI::new("c1+-.:is valid").is_ok());
    }
}