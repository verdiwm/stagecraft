use std::future::Future;

use futures_core::Stream;

use super::{context::Context, mailbox::HasMailbox};

pub trait Actor: HasMailbox + Send {
    type Init: Send + 'static;

    fn init(init: Self::Init, ctx: &mut Context<Self>) -> impl Future<Output = Self> + Send;

    fn handle_message(
        &mut self,
        msg: Self::Message,
        ctx: &mut Context<Self>,
    ) -> impl Future<Output = ()> + Send;

    fn on_stop(&mut self, _ctx: &mut Context<Self>) -> impl Future<Output = ()> + Send {
        async {}
    }
}

pub trait StreamActor: Actor {
    type Event: Send + 'static;
    type Stream: Stream<Item = Self::Event> + Send + 'static;

    fn create_stream(
        &mut self,
        ctx: &mut Context<Self>,
    ) -> impl Future<Output = Self::Stream> + Send;

    fn handle_event(
        &mut self,
        event: Self::Event,
        ctx: &mut Context<Self>,
    ) -> impl Future<Output = ()> + Send;
}

/// An actor that runs on a dedicated OS thread with a LocalSet.
/// Use this for actors whose state is !Send (e.g., they hold libinput contexts).
/// Only `Init` needs to be `Send` — it crosses the thread boundary.
/// `init()` runs on the local thread and returns the (possibly !Send) actor.
pub trait LocalActor: HasMailbox {
    type Init: Send + 'static;

    fn init(init: Self::Init, ctx: &mut Context<Self>) -> impl Future<Output = Self>;

    fn handle_message(
        &mut self,
        msg: Self::Message,
        ctx: &mut Context<Self>,
    ) -> impl Future<Output = ()>;

    fn on_stop(&mut self, _ctx: &mut Context<Self>) -> impl Future<Output = ()> {
        async {}
    }
}

pub trait LocalStreamActor: LocalActor {
    type Event: 'static;
    type Stream: Stream<Item = Self::Event> + 'static;

    fn create_stream(&mut self, ctx: &mut Context<Self>) -> impl Future<Output = Self::Stream>;

    fn handle_event(
        &mut self,
        event: Self::Event,
        ctx: &mut Context<Self>,
    ) -> impl Future<Output = ()>;
}
