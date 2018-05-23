use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::collections::hash_map::DefaultHasher;
use std::hash::Hasher;

use soft_ascii_string::SoftAsciiString;

use common::error::EncodingError;
use headers::components::{MessageId, ContentId, Domain};
use ::context::MailIdGenComponent;


static MAIL_COUNTER: AtomicUsize = AtomicUsize::new(0);

// Message-Id:
//     U.C.YYYY-MM-DD@domain
//     ↑ ↑   ↑   ↑  ↑
//     | |   |   |  \- day of month 2-digeds always
//     | |   |   \- month 2-digeds always
//     | |   \- year 4-digeds always
//     | \- counter increases with each call to get_message_id
//     \- unique part given when starting a new instance

// Content-Id:
//     U.C.YYYY-MM-DD.C@domain
//     ↑ ↑   ↑   ↑  ↑ ↑
//     | |   |   |  | \- counter increases with each call to content_id
//     | |   |   |  |    resets with each call to message id
//     | |   |   |  \- day of month 2-digeds always
//     | |   \- year 4-digeds always
//     | \- counter increases with each call to get_message_id
//     \- unique part given when starting a new instance

#[derive(Debug)]
pub(crate) struct UniqueParts {
    domain: SoftAsciiString,
    part_unique_in_domain: SoftAsciiString
}

/// a simple id gen implementation
///
/// Message Id's are constructed through a template like `{u}.{c}@{domain}`
/// where `u` is the `part_unique_in_domain` used for constructing and
/// `c` is a internal counter. Note that it is actually a global counter
/// assuring that no two message id's generated during the same program
/// execution are the same (and as extension of it the `part_unique_in_domain`
/// has to change with each program execution/instance and has to be world unique
/// for all applications using the given domain).
///
/// The message id stays the same until `for_new_mail` is called in which
/// case the returned instance will have a new message id.
///
/// Content Id's use the template `{u}.{c}.{mc}@{domain}`, where
/// `u` and `c` are the same as for the message id, and mc is
/// an internal counter increased every time `generate_content_id` is
/// called.
//IMPORTANT DO NOT IMPLEMENT `Clone`, (just put it in a Arc to "clone" it)
#[derive(Debug)]
pub struct SimpleIdGen {
    unique_parts: Arc<UniqueParts>,
    use_mail_id: usize,
    cid_counter: AtomicUsize,
}

impl SimpleIdGen {

    pub fn new(domain: Domain, part_unique_in_domain: SoftAsciiString) -> Result<Self, EncodingError> {
        let domain = domain.into_ascii_string()?;
        let id_gen = SimpleIdGen::from_arc(Arc::new(UniqueParts {
            domain,
            part_unique_in_domain
        }));
        Ok(id_gen)
    }

    pub(crate) fn from_arc(unique_parts: Arc<UniqueParts>) -> Self {
        let use_mail_id = MAIL_COUNTER.fetch_add(1, Ordering::AcqRel);
        SimpleIdGen {
            use_mail_id,
            unique_parts,
            cid_counter: AtomicUsize::new(0),
        }
    }

    fn gen_next_content_id_num(&self) -> usize {
        self.cid_counter.fetch_add(1, Ordering::AcqRel)
    }
}

impl MailIdGenComponent for SimpleIdGen {

    //this is normally only called once so we don't cache it's result
    fn get_message_id(&self) -> MessageId {
        let msg_id = format!("{u}.{c}@{domain}",
            u=self.unique_parts.part_unique_in_domain,
            c=self.use_mail_id,
            domain=self.unique_parts.domain
        );

        MessageId::from_unchecked(msg_id)
    }

    fn generate_content_id(&self) -> ContentId {
        let new_cid = self.gen_next_content_id_num();

        let msg_id = format!("{u}.{c}.{mc}@{domain}",
            u=self.unique_parts.part_unique_in_domain,
            c=self.use_mail_id,
            mc=new_cid,
            domain=self.unique_parts.domain
        );

        ContentId::from_unchecked(msg_id)
    }

    fn for_new_mail(_self: &Arc<Self>) -> Arc<Self> {
        // this will "reset" the inner count for content id
        // it still won't lead to a collision as it also will
        // use a new message id
        Arc::new(SimpleIdGen::from_arc(_self.unique_parts.clone()))
    }
}

/// a id gen where the left part of the message/content id is a hash
///
/// This is a wrapper around `SimpleIdGen` using hashing instead of
/// string templates, while the produced message/content id's have
/// less semantic meaning they also expose less information, e.g.
/// if `SimpleIdGen` is used someone receiving your mails could under
/// some circumstances roughly guess how many mails where send between
/// two mails received by that person.
///
/// The hash is crated from the `part_unique_in_domain` and the used
/// message id. For content id's the used content id is included too.
///
/// Both the used message and content id are currently internal counters,
/// and the same as in `SimpleIdGen`.
#[derive(Debug)]
pub struct HashedIdGen {
    id_gen: SimpleIdGen
}

impl HashedIdGen {

    pub fn new(domain: Domain, part_unique_in_domain: SoftAsciiString) -> Result<Self, EncodingError> {
        let id_gen = SimpleIdGen::new(domain, part_unique_in_domain)?;
        Ok(HashedIdGen { id_gen })
    }
}

impl MailIdGenComponent for HashedIdGen {

    //this is normally only called once so we don't cache it's result
    fn get_message_id(&self) -> MessageId {
        let mut hasher = DefaultHasher::new();
        hasher.write(self.id_gen.unique_parts.part_unique_in_domain.as_bytes());
        hasher.write_usize(self.id_gen.use_mail_id);
        let hash = hasher.finish();

        let msg_id = format!("{:x}@{domain}", hash, domain=self.id_gen.unique_parts.domain);
        MessageId::from_unchecked(msg_id)
    }

    fn generate_content_id(&self) -> ContentId {
        let mut hasher = DefaultHasher::new();
        hasher.write(self.id_gen.unique_parts.part_unique_in_domain.as_bytes());
        hasher.write_usize(self.id_gen.use_mail_id);
        hasher.write_usize(self.id_gen.gen_next_content_id_num());
        let hash = hasher.finish();

        let msg_id = format!("{:x}@{domain}", hash, domain=self.id_gen.unique_parts.domain);
        ContentId::from_unchecked(msg_id)
    }

    fn for_new_mail(_self: &Arc<Self>) -> Arc<Self> {
        Arc::new(HashedIdGen { id_gen: SimpleIdGen::from_arc(_self.id_gen.unique_parts.clone()) })
    }
}

#[cfg(test)]
mod test {

    macro_rules! test_id_gen {
        ($name:ident) => (
            mod $name {
                #![allow(non_snake_case)]

                use std::sync::Arc;
                use std::collections::HashSet;
                use soft_ascii_string::SoftAsciiString;
                use headers::components::Domain;
                use headers::HeaderTryFrom;

                //NOTE: this is a rust bug, the import is not unused
                #[allow(unused_imports)]
                use ::context::MailIdGenComponent;
                use super::super::$name;

                fn setup() -> Arc<$name> {
                    let unique_part = SoftAsciiString::from_string_unchecked("bfr7tz4");
                    let domain = Domain::try_from("fooblabar.test").unwrap();
                    Arc::new($name::new(domain, unique_part).unwrap())
                }

                mod get_message_id {
                    use super::*;

                    #[test]
                    fn should_return_the_same_id() {
                        let id_gen = setup();

                        let msg_id = id_gen.get_message_id();
                        let msg_id2 = id_gen.get_message_id();

                        assert_eq!(msg_id, msg_id2);
                    }

                    #[test]
                    fn should_change_for_new_mails() {
                        let id_gen = setup();
                        let msg_id = id_gen.get_message_id();

                        let other_id_gen = $name::for_new_mail(&id_gen);
                        let omsg_id = other_id_gen.get_message_id();

                        assert_ne!(msg_id, omsg_id);
                    }

                }

                mod for_new_mail {
                    use super::*;

                    #[test]
                    fn should_not_change_the_current_id_gen() {
                        let id_gen = setup();
                        let msg_id = id_gen.get_message_id();

                        let other_id_gen = $name::for_new_mail(&id_gen);
                        assert_eq!(msg_id, id_gen.get_message_id());

                        let _omsg_id = other_id_gen.get_message_id();
                        assert_eq!(msg_id, id_gen.get_message_id());
                    }
                }

                mod generate_content_id {
                    use super::*;

                    #[test]
                    fn should_always_return_a_new_id() {
                        let id_gen = setup();
                        let mut cids = HashSet::new();
                        for _ in 0..20 {
                            assert!(cids.insert(id_gen.generate_content_id()))
                        }
                    }
                }

            }
        );
    }

    test_id_gen!{SimpleIdGen}
    test_id_gen!{HashedIdGen}
}