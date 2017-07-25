-- FIXME AsciiString => HeaderName
-- NOT HERE Content-Header-Extension  |AsciiString | Unstructured|,
-- NOT HERE Other  |AsciiString | Unstructured|,

RFC   | Name                      | Rust-Type         | Comment
------|---------------------------|-------------------|----------------------------
5322  | Date                      | DateTime          |
      | From                      | AddressList       |
      | Sender                    | Mailbox           |
      | Reply-To                  | MailboxList       |
      | To                        | MailboxList       |
      | Cc                        | MailboxList       |
      | Bcc                       | OptMailboxList    |
      | Message-ID                | MessageID         |
      | In-Reply-To               | MessageIDList     |
      | References                | MessageIDList     |
      | Subject                   | Unstructured      |
      | Comments                  | Unstructured      |
      | Keywords                  | PhraseList        |
      | Resent-Date               | DateTime          |
      | Resent-From               | AddressList       |
      | Resent-Sender             | Mailbox           |
      | Resent-To                 | MailboxList       |
      | Resent-Cc                 | MailboxList       |
      | Resent-Bcc                | OptAddressList    |
      | Resent-Msg-ID             | MessageID         |
      | Return-Path               | Path              |
      | Received                  | ReceivedToken     |
------|---------------------------|-------------------|---------------------------
2045  | Content-Type              | Mime              |
      | Content-ID                | MessageID         |
      | Content-Transfer-Encoding | TransferEncoding  |
      | Content-Description       | Text              | is Text the same as unstructured ? older
      |                           |                   | RFC has text instead of unstructured?
------|---------------------------|-------------------|---------------------------
2183  | Content-Disposition       | Disposition       | proposed standard (obsoltets rfc 1806)
------|---------------------------|-------------------|---------------------------



------ "others" ----
-- e.g. see https://www.cs.tut.fi/~jkorpela/headers.html
--Delivered-To   |loop detection|
--User-Agent   |client software used by orginator|
--Abuse-Reports-To   |inserted by some servers|
--X-Envelop-From  |Address|   |sender in the envelop copied into the body|
--X-Envelop-To  |Address|   |again envelop information moved into body|
--X-Remote-Addr   |from html|
--
------Proposed Standard----
--RFC 1766
-- Content-Language  |LanguageTag|
--RFC 1864
-- Content-MD5  |Base64|
--
------Experimental--------
--RFC 1806   |attachment of inline|
-- Content-Disposition  |Dispositions|
--RFC 1327 & 1911
-- Importance
-- Sensitivity
--RFC 1154 & 1505
-- Encoding
--
------Not Standad ------
--RFC 1036
-- FollowupTo  |??MessageID|
--RFC 1036   |count of lines|
-- Lines  |usize|
--RFC ????
-- Status  |U/R/O/D/N|   |should NEVER EVER be generate for a mail to send, use by some mail delivery systems INTERNAL ONLY|
--
--
------Not Standard Discouraged----
--ContentLength  |usize|   |do never generate content length header in a mail you send|