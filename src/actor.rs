use std::future::Future;

use futures_core::Stream;

use super::{context::Context, mailbox::HasMailbox};

/// An asynchronous, `Send`-safe actor.
///
/// Implement this for actors whose state is `Send`. The runtime calls [`init`] once,
/// then loops calling [`handle_message`] for each message, and finally calls
/// [`on_stop`] before the task exits.
///
/// For `!Send` state use [`LocalActor`]. To also receive stream events use [`StreamActor`].
///
/// [`init`]: Actor::init
/// [`handle_message`]: Actor::handle_message
/// [`on_stop`]: Actor::on_stop
pub trait Actor: HasMailbox + Send {
    /// Initialization parameter. Must be `Send + 'static` since it may cross task boundaries.
    type Init: Send + 'static;

    /// Construct the actor. Called once before the message loop starts.
    fn init(init: Self::Init, ctx: &mut Context<Self>) -> impl Future<Output = Self> + Send;

    /// Process a single incoming message.
    fn handle_message(
        &mut self,
        msg: Self::Message,
        ctx: &mut Context<Self>,
    ) -> impl Future<Output = ()> + Send;

    /// Called after the message loop exits. Default is a no-op.
    fn on_stop(&mut self, _ctx: &mut Context<Self>) -> impl Future<Output = ()> + Send {
        async {}
    }
}

/// An [`Actor`] that also processes events from a [`Stream`].
///
/// After [`Actor::init`], the runtime calls [`create_stream`] once and then selects
/// over both the message inbox and the event stream. The actor stops when the
/// cancellation token fires or the stream is exhausted.
///
/// [`create_stream`]: StreamActor::create_stream
/// [`Stream`]: futures_core::Stream
pub trait StreamActor: Actor {
    /// The item type produced by the event stream.
    type Event: Send + 'static;

    /// The event stream type.
    type Stream: Stream<Item = Self::Event> + Send + 'static;

    /// Create the event stream. Called once after [`Actor::init`].
    fn create_stream(
        &mut self,
        ctx: &mut Context<Self>,
    ) -> impl Future<Output = Self::Stream> + Send;

    /// Process a single event from the stream.
    fn handle_event(
        &mut self,
        event: Self::Event,
        ctx: &mut Context<Self>,
    ) -> impl Future<Output = ()> + Send;
}

/// An actor that runs on a dedicated OS thread with a [`LocalSet`].
///
/// Use this for actors whose state is `!Send` (e.g. they hold a libinput context).
/// Only [`Init`] crosses the thread boundary and must therefore be `Send`.
/// [`init`] runs on the local thread and may return a `!Send` actor.
///
/// Spawn with [`Context::spawn_stream_local`]. To also handle a stream, implement
/// [`LocalStreamActor`] instead.
///
/// [`LocalSet`]: tokio::task::LocalSet
/// [`Init`]: LocalActor::Init
/// [`init`]: LocalActor::init
pub trait LocalActor: HasMailbox {
    /// Initialization parameter. Must be `Send + 'static` to cross the thread boundary.
    type Init: Send + 'static;

    /// Construct the actor on the local thread. Called once before the message loop.
    fn init(init: Self::Init, ctx: &mut Context<Self>) -> impl Future<Output = Self>;

    /// Process a single incoming message.
    fn handle_message(
        &mut self,
        msg: Self::Message,
        ctx: &mut Context<Self>,
    ) -> impl Future<Output = ()>;

    /// Called after the message loop exits. Default is a no-op.
    fn on_stop(&mut self, _ctx: &mut Context<Self>) -> impl Future<Output = ()> {
        async {}
    }
}

/// A [`LocalActor`] that also processes events from a [`Stream`].
///
/// The stream's item type does not need to be `Send`. Spawn with
/// [`Context::spawn_stream_local`].
///
/// [`Stream`]: futures_core::Stream
pub trait LocalStreamActor: LocalActor {
    /// The item type produced by the event stream.
    type Event: 'static;

    /// The event stream type.
    type Stream: Stream<Item = Self::Event> + 'static;

    /// Create the event stream. Called once after [`LocalActor::init`].
    fn create_stream(&mut self, ctx: &mut Context<Self>) -> impl Future<Output = Self::Stream>;

    /// Process a single event from the stream.
    fn handle_event(
        &mut self,
        event: Self::Event,
        ctx: &mut Context<Self>,
    ) -> impl Future<Output = ()>;
}
