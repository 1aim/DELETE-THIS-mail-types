//! Utilities.
//!
//! Contains everything which doesn't has a better place
//! to be put in.
use std::marker::Send;
use std::fmt::Debug;
#[cfg(test)]
use std::{
    time::Duration,
    thread
};

use chrono;
use futures::Future;
#[cfg(test)]
use futures::sync::oneshot;


/// Type alias for an boxed future which is Send + 'static.
pub type SendBoxFuture<I, E> = Box<Future<Item=I, Error=E> + Send + 'static>;

/// Returns the current data time.
pub fn now() -> chrono::DateTime<chrono::Utc> {
    chrono::Utc::now()
}

/// Trait to allow const `bool` values in generics.
pub trait ConstSwitch: Debug + Copy + Send + Sync + 'static {
    const ENABLED: bool;
}

/// Struct implementing `ConstSwitch` with `ENABLED = true`.
#[derive(Debug, Copy, Clone)]
pub struct Enabled;
impl ConstSwitch for Enabled { const ENABLED: bool = true; }
/// Struct implementing `ConstSwitch` with `ENABLED = false`.
#[derive(Debug, Copy, Clone)]
pub struct Disabled;
impl ConstSwitch for Disabled { const ENABLED: bool = false; }


#[cfg(test)]
pub(crate) fn timeout( s: u32, ms: u32 ) -> oneshot::Receiver<()> {
    let (timeout_trigger, timeout) = oneshot::channel::<()>();

    thread::spawn( move || {
        thread::sleep( Duration::new( s as u64, ms * 1_000_000) );
        //we do not care if it faile i.e. the receiver got dropped
        let _ = timeout_trigger.send( () );
    });

    timeout
}
