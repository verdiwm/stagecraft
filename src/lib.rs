#![deny(missing_docs)]
//! Tokio-based actor framework.
//!
//! Stagecraft provides four actor traits and a [`Context`] type that orchestrates
//! their lifecycle. Actors communicate through typed message channels via [`Handle`].
//!
//! ## Actor Variants
//!
//! | Trait | `Send` required | Event stream |
//! |---|---|---|
//! | [`Actor`] | yes | no |
//! | [`StreamActor`] | yes | yes |
//! | [`LocalActor`] | no | no |
//! | [`LocalStreamActor`] | no | yes |
//!
//! Use [`Actor`] for the common case. Use [`LocalActor`] when actor state is `!Send`
//! (e.g. it holds a libinput context). Append `Stream` to also react to a [`Stream`].
//!
//! [`Stream`]: futures_core::Stream

mod actor;
mod context;
mod handle;
mod mailbox;

pub use actor::{Actor, LocalActor, LocalStreamActor, StreamActor};
pub use context::{Context, spawn};
pub use handle::{ActorDead, Handle};
pub use mailbox::HasMailbox;
pub use stagecraft_macros::message;
