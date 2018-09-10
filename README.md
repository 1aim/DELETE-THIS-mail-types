
# mail-types

**Provides the core mail type `Mail` for the `mail` crate.**

---

This crate provides the type called `mail` as well as ways
to create it. It also provides the builder context interface
and the `Resource` type, which is used to represent mail bodies.
Especially such which are attachments or embedded images.


# Example

```rust
extern crate futures;
// Note that the `mail` crate provides a facade re-exporting
// all relevant parts.
extern crate mail_types;
extern crate mail_common;
#[macro_use]
extern crate mail_headers;

use std::str;
use futures::Future;

use mail_common::MailType;

// In the facade this is the `headers` module.
use mail_headers::{
    headers::*,
    header_components::Domain
};

// In the facade this types (and the default_impl module)
// are also exposed at top level
use mail_types::{
    Mail,
    default_impl::simple_context,
    error::MailError
};

fn print_some_mail() -> Result<(), MailError> {
    // Domain will implement `from_str` in the future,
    // currently it doesn't have a validator/parser.
    // So this will become `"example.com".parse()`
    let domain = Domain::from_unchecked("example.com".to_owned());
    // Normally you create this _once per application_.
    let ctx = simple_context::new(domain, "xqi93".parse().expect("we know it's ascii"))
        .expect("this is basically: failed to get cwd from env");

    let mut mail = Mail::plain_text("Hy there! 😁");
    mail.insert_headers(headers! {
        _From: [("I'm Awesome 😁", "bla@examle.com")],
        _To: ["unknow@example.com"],
        Subject: "Hy there message 😁"
    }?);

    // We don't added any think which needs loading but we could have
    // and all of it would have been loaded concurrent and async.
    let encoded = mail.into_encodeable_mail(ctx.clone())
        .wait()?
        .encode_into_bytes(MailType::Ascii)?;

    let mail_str = str::from_utf8(&encoded).unwrap();
    println!("{}", mail_str);
    Ok(())
}

fn main() {
    print_some_mail().unwrap()
}
```


Documentation can be [viewed on docs.rs](https://docs.rs/mail-types)
(once it is published).


## License

Licensed under either of

 * Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
 * MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any
additional terms or conditions.
