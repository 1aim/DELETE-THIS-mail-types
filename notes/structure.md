
# Naming differences

(note that `foo/bar` means crate `foo` with feature `bar` enabled)

- `mail-codec` => `mail-encode`
- `mail-codec-composition` => `mail-encode-compose`
- `mail-codec-composition/smtp` => `mail-tokio-smtp/encode`
- `mail-headers` => `mail-headers/encode`
- `mail-common` => `mail-core/encode`


# Dependency Chart

```ascii
                                   mail
                 __________________/  \______
  mail-tokio-smtp*    /        /             \
     |     |         /        /               \
     |     mail-compose      /                 \
     |             |        /                   \
     \-----------mail-encode____           _____mail-parse <--\
                     |          \         /       |            |
                     |          mail-headers*     |       move HeaderMap and other
                      \_________    |     _______/        header stuff here
                                \   |    /                             /\
                                 \  |   /                               |
                                 mail-common*        --->current place-/
```

crates marked with a * have both parser and encoder specific parts,
which are opt-in/-out through features.

# Descriptions

## Mail-Core

This crate provides parts used by all other parts using
the `encode`/`parse` features encoding/parsing specific
parts are enabled/disabled. (Currently it's just encoding).

Parts included are the EncodingBuffer, HeaderMap, Header traits,
encoding traits, bindings to other encoding libraries like
base64, percent-encode and similar.

## Mail-Headers

A create providing implementations for the most
common headers. Like `mail-core` encoding/parsing specific
parts can be enabled/disabled using the `encode`/`decode`
feature. A Header basically consists of a name and a Header component,
which can be encoded and represents the header field.

Many of those components can be reused at different
points, e.g. `Mailbox`/`MailboxList`/`Email`/`Phrase`/...


## Mail-Encode

This crate provides a basic mechanism to create and
encode mails with includes a Mail types which can
represent any form of mails, from simple text mails
to complex multipart mails. While this API can be
used to represent any mail it is not nessesary the
most convinient to use, as it requires the user to
handle aspects like e.g. which multipart content type
is used, and how they are structured.

Additional to the Mail struct it provides a mechanism
to encode the Mail struct with the EncodingBuffer provided
by `mail-core` produceing a mail in form of a
string, ascii-string or bytes a required by the consumer.
(the encoder does know about 7bit, 8bit and utf8 i.e.
internationalized mails).

As a mail can contain all kinds of embedded data (e.g.
atachments, embeded images in a html mail etc.). This
create also provides a way to handle such resources
as part of a mail.


## Mail-Encode-Compose

Is build on-top of `mail-encode` providing traits and mechanism,
to bind in template engines, allowing the API consumer to
create mails based on from, to, subject, template_id and
template_data. The template engine only has to provide
a number of alternate bodies with embeddings/attachments
caused by them. The mail-composition crate takes care of
composing them into a mail multipart tree
(using `multipart/related`,`multipart/mixed`, etc.)

Currently `mail-composition` does not have explicit support for
e.g. encryption and some other "more special" multipart
bodies. Nevertheless by using the API provided by Mail this
mail can still be created, they do just not jet bind with
the template engine by default.

The crate also provides a pre-build template engine through a
feature which just needs another template engine to render
mail content bodies (e.g. text/html) and handles all the
other parts, like e.g. providing a logo for an HTML mail
as an embedding (as a side note generation of CID's and
providing them to the template data is already handled)

# Mail

A facade for all `mail-*` crates, providing a `encode`
`parse` feature to enable/disable the encoding/parsing
parts, and more fine grained features for more fain
grained control over what is included. This
takes advantage of rust features being additive, e.g.
if one dependency includes `mail/encode` and one `mail/decode`
than rust will only include one time `mail` with both
`encode` and `decode` enabled.

(this crate does not yet exist)


# Mail-Tokio-Smtp

This crate will provide bindings between the mail
crates and a crate providing tokio-smtp support.
While smtp is mainly a transport protocol this
crate provides bindings to make thinks easy to
use and setup, e.g. the `mail-encode-compose`
library does provide a method `compose_mail(MailSendData) -> Result<Mail>`
(called on a combination of Context and TemplateEngine),
but if you want to use this as a request in a smtp
service to send a mail, there is quite a bit of bind
code doing everything from calling compose_mail, making
it encodable, encoding it and sending the right smtp
commands.

Currently this functionality lives in `mail-encode-compose`
under the `smtp` feature and use `tokio-smtp` as binding
library and is limited to binding with the mechanism provided
in `mail-encode-compose` but not general `Mail` structs (through
is will change in the future)


# Mail-parse

A crate providing functionality for parsing mails, it
will use a combination of `Arc`/`Rc` and `OwningRef`
to allow a zero-copy parsing of mails, nevertheless
as mail normaly do contain parts further encoded in
e.g. base64 it will provide a mechanism where parts
of the mail can be shadowed by a decoded version
of the part generated on-demand/lazily.

**This crate currently does not exist, and focus lies
on the encoding based crates**

Through the crate structure is setup to include parsing
specific parts in the future.