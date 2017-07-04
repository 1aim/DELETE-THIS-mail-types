use ascii::AsciiStr;
use codec::{ SmtpDataEncodable, SmtpDataDecodable };


pub mod shared;

pub mod components;

mod address_list;
pub use self::address_list::{ AddressList, OptAddressList };

mod address;
pub use self::address::Address;

mod unstructured;
pub use self::unstructured::Unstructured;

mod message_id;
pub use self::message_id::{ MessageID, MessageIDList };


mod header_name;
pub use self::header_name::HeaderName;

mod date_time;
pub use self::date_time::DateTime;