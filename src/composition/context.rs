use std::sync::Arc;
use std::ops::Deref;
use std::path::Path;
use std::borrow::Cow;

use futures::Future;
use utils::SendBoxFuture;

use core::error::*;
use context::{ FileLoader, RunElsewhere, BuilderContext };
use mheaders::components::{ Mailbox,  MessageID };

pub struct MailSendContext {
    pub from: Mailbox,
    pub to: Mailbox,
    pub subject: String
}

pub trait ContentIdGen {
    fn new_content_id( &self ) -> Result<MessageID>;
}

pub trait Context: BuilderContext + ContentIdGen + Send + Sync {}

impl<T> Context for T where T: BuilderContext + ContentIdGen + Send + Sync {}

impl<T: ContentIdGen> ContentIdGen for Arc<T> {
    fn new_content_id( &self ) -> Result<MessageID> {
        self.deref().new_content_id()
    }
}





pub struct ComposedContext<CIG, BC> {
    id_gen: CIG,
    builder_context: BC
}

impl<CIG, BC: BuilderContext> FileLoader for ComposedContext<CIG, BC> {
    type FileFuture = <BC as FileLoader>::FileFuture;
    fn load_file( &self, path: Cow<'static, Path> ) -> Self::FileFuture {
        self.builder_context.load_file( path )
    }
}

impl<CIG, BC: BuilderContext> RunElsewhere for ComposedContext<CIG, BC> {
    fn execute<F>( &self, fut: F) -> SendBoxFuture<F::Item, F::Error>
        where F: Future + Send + 'static,
              F::Item: Send+'static,
              F::Error: Send+'static
    {
        self.builder_context.execute( fut )
    }
}


impl<CIG: ContentIdGen, BC> ContentIdGen for ComposedContext<CIG, BC> {

    fn new_content_id( &self ) -> Result<MessageID> {
        self.id_gen.new_content_id()
    }
}



