
# Outer Most interface

something like a Mailer which might implement tokio_servie::Service (if
so multiple parameters are wrapped into a tupple)

mailer contains information like `from`

`mailer.send_mails( recipients_data, mail_gen )`

where recipients_data is a iterable mapping from address to recipient specific data,
e.g. `Vec<(Address, Data)>`

and mail_gen is something like `trait MailGen { fn gen_mail( from, to, data, bits8support ) ->  MailBody; }`
 
`MailBody` is not `tokio_smtp::MailBody` but has to implement nessesray contraints,
(e.g. implemnting `toki_smtp::IntoMailBody` not that for the beginning this will be
hard encoded but later one a generic variation allowing `smtp` to be switched out
by something else is also possible`)

MailGen implementations are not done by hand but implemented ontop of something
like a template spec e.g. `struct TemplateSpec { id_template: TemplateId, additional_appendixes: Vec<Appendix> }`

Where `TemplateId` can is e.g. `reset_link` leading to the creation of a `html` with alternate `plain`
mail iff there is a `reset_link.html` and a `reset_link.plain` template. A `reset_link.html.data` 
folder could be used to define inline (mime related) appendixes like embedded images,
but we might want to have a way to define such embeddigns through the data (
E.g. by mapping `Data => TemplateEnginData` and replacing `EmbeddedFile` variations
by a new related id and adding the `EmbeddedFile(data)` data to the list of embeddings)



# List of parts possible non-ascii and not ascii encodable

- local-part (address/addr-spec/local-part)

# Limitations

Line length limit:

SHOULD be no more than 78 chars (excluding CRLF!)
MUST NOT be more than 998 chars (excluding CRLF)

# Orphan `\n`,`\r`

MUST NOT occur in header (except for folding)
MUST NOT occur in body (except for newline)

## Header specific limitations

- encoded word max length of 75 chars
- spaces around encoed words are ignored??


# Email Address part (a@b.e)

- there is a `domain-literal` version which does use somthing like `[some_thing]`,
  we can use puny code for converting domains into ascii but probably can't use
  this with `domain-literal`'s
  
- `local-part` is `dot-atom` which has leading and trailing `[CFWS]` so comments are alowed

- MessageId uses a email address like syntax but without supporting spaces/comments
  

# MIME

fields containing mime types can have parameters with a `<type>; key=value` style
this is mainly used for `multipart/mixed; boundary=blablabla` and similar.

You have to make sure the boundary does not appear in any of the "sub-bodies",
this is kinda easy for bodies with e.g. content transfer encoding Base64,
but can be tricky in combination with some other content as normal text
can totally contain the boundary. To prevent this:

- use long boundary strings
- encode the body with base64 even if it's "just" ascii
    - OR check the content and encode parts of it if necessary

you can have multipart in multipart creating a tree,
make sure you don't mix up the boundaries


A body part does not have to have any headers, assume default values if
there is no header, bodies which have no header _have to start with a
blank line_ separating 0 headers from the body.

Header fields of bodies which do not start with `Content-` _are ignored_!

Contend types:

- `mixed`, list of sub-bodies with mixed mime types, might be displayed inline or as appendix
    - use >>`Content-Disposition` (RFC 2183)<< to controll this, even through it's not standarized yet (or is it by now?)
    - default body mime type is `text/plain`
- `digest` for combining muliple messages of content type `message/rfc822`
    - e.g. `(multipar/mixed ("table of content") (multipart/digest "message1", "message2"))`
    - `message` (mainly `message/rfc822`) contains _another_ email, e.g. for digest
        - wait is there a `multipart/message`?? proably not!
- `alternative` multiple alternative versions of the "same" information
    - e.g. `(multipart/alternative (text/plain ...) (text/html ...))`
    - _place preferred form last!_ (i.e. increasing order of preference)
    - interesting usage with `application/X-FixedRecord`+`application/octet-stream`
- `related` all bodies are part of one howl, making no (less) sense if placed alone
    - the first part is normally the entry point, but this can be chaged through parameters
        - (only relevant for parsing AND interpreting it, but not for generating as we can always use the default)
    - Content-ID is used to specify a id on each body respectivly which can be used to refer to it (e.g. in HTML)
        - in html use e.g. `<img src="cid:the_content_id@goes.here>....</img>`
    - example is `(multipart/relat (text/html ...) (image/jpeg (Content-ID <bala@bal.bla>) ...))` for embedding a image INTO a HTML mail
- `report`
- `signed` (body part + signature part)
- `encrypted` (encryption information part + encrypted data (`application/octet-stream`))
- `form-data`
- `x-mixed-replace` (for server push, don't use by now there are bether ways)
- `byteranges`
    

Example mail structure:

```
(multipart/mixed 
    (multipart/alternative
        (text/plain ... ) 
        (multipart/related 
            (text/hmtl ... '<img src="cid:contentid@1aim.com"></img>' ... ) 
            (image/png (Content-ID <contentid@1aim.com>) ... ) 
            ... ))
    (image/png (Content-Disposition attachment) ...)
    (image/png (Content-Disposition attachment) ...))
```


# Attatchment

proposed filenames for attachments can be given through parameters of the disposition header

it does not allow non ascii character there!

see rfc2231 for more information, it extends some part wrt.:
    
- splitting long parameters (e.g. long file names)
- specifying language and character set
- specifying language for encoded words

# Encoded Words

extended by rfc2231

additional limits in header fields

header containing encoded words are limited to 76 bytes

a "big" text chunk can be split in multiple encoded words seperated by b'\r\n '

non encoded words and encoded words can apear in the same header field, but
must be seperate by "linear-white-space" (space) which is NOT removed when
decoding encoded words 

encoded words can appear in:

- `text` sections where `text` is based on RFC 822! (e.g. Content-Description )
    - in context of RFC 5322 this means `unstructured` count's as text
- `comments` (as alternative to `ctext`,`quoted-pair`,`comment`
- `word`'s within a `phrase`

**Therefor it MUST NOT appear in any structured header field except withing a `comment` or `phrase`!**

**You have to encode text which looks like an encoded word**



limitations:

- in comment's no ')',')' and '"'
- in headers no ' '


# Other

there is no `[CFWS]` after the `:` in Header fields,
but most (all?) of the parts following them are allowed
to start with a `[CFWS]`. (exception is unstructured where
a `CFWS` can be allowed but also MIGHT be part of the
string)

CFWS -> (un-) foldable whitespace allowing comments
FWS -> (un-) foldable whitespace without comments


# Relevant RFCs
5321, 5322, 6854, 3492, 2045, 2046, 2047, 4288,  4289, 2049, 6531, 5890

make sure to not use the outdated versions


# Parsing Notes

be strict when parsing (e.g. only ws and printable in subject line)

if "some other" strings should still be supported do not do zero
copy, but instead add the data to a new buff _replacing invalid
chars with replacement symbol or just stripping them_


# Non-utf8 Non-Ascci bytes in Mail body

The mail body can contain non-utf8, non-ascii data (e.g.
utf16 data, images etc.) WITHOUT base64 encoding if
8BITMIME is supported (note there is also BINARY and CHUNKING)

smtp still considers _the bytes_ corresponding to CR LF and DOT special.

- there is a line length limit, lines terminate with b'CRLF'
- b'.CRLF' does sill end the body (if preceeded by CRLF, or body starts with it)
    - so dot-staching is still done on protocol level
    


## Hot to handle `obs-` parsings

we have to be able to parse mails with obsolete syntax (theoretically)
but should never genrate such mails, the encder excepts its underlying
data to be correct, but it might not be if we directly place `obs-`
parsed data there. For many parts this is no problem as the
`obs-` syntax is a lot about having FWS at other positions,
_between components_ (so we won't have a problem with the encoder).
Or some additional obsolete infromations (which we often/allways can just
"skip" over). So we have to check if there are any braking cases and if
we have to not zero copy them when parsing but instead transform them
into a valide representation, in worst case we could add a `not_encodable`
field to some structs.

# TODO
check if some parts are empty and error if encode is calde on them
e.g. empty domain
