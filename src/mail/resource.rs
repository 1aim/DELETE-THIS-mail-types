use std::path::PathBuf;

use mime::Mime;
use futures::future::BoxFuture;

use utils::Buffer;
use error::Error;

#[derive(Debug)]
pub enum Resource {
    File {
        //FIXME make it optional and use mime sniffing
        // sniff with magical number and file ending
        mime: Mime,
        path: PathBuf,
        alternate_name: Option<String>
    },
    Buffer( Buffer ),
    Future( BoxFuture<Item=Buffer, Error=Error> )
}
