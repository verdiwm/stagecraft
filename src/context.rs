use std::future::Future;

use tokio::{
    pin,
    runtime::Handle as RuntimeHandle,
    sync::{mpsc, oneshot},
    task::LocalSet,
};
use tokio_stream::StreamExt;
use tokio_util::{sync::CancellationToken, task::TaskTracker};

use crate::{LocalStreamActor, StreamActor, actor::Actor, handle::Handle, mailbox::HasMailbox};

pub struct Context<A: HasMailbox> {
    handle: Handle<A>,
    token: CancellationToken,
    tracker: TaskTracker,
    original_runtime: RuntimeHandle,
}

impl<H: HasMailbox> Context<H> {
    pub fn handle(&self) -> Handle<H> {
        self.handle.clone()
    }

    pub fn shutdown(&self) {
        self.token.cancel();
    }

    pub fn child_token(&self) -> CancellationToken {
        self.token.child_token()
    }

    pub fn spawn<A: Actor>(&self, init: A::Init) -> Handle<A> {
        spawn_internal::<A>(
            self.token.child_token(),
            &self.tracker,
            init,
            self.original_runtime.clone(),
        )
    }

    pub fn spawn_stream<C: super::actor::StreamActor>(&self, init: C::Init) -> Handle<C> {
        spawn_stream_internal::<C>(
            init,
            self.token.child_token(),
            &self.tracker,
            self.original_runtime.clone(),
        )
    }

    pub fn spawn_stream_local<C: LocalStreamActor>(&self, init: C::Init) -> Handle<C> {
        spawn_stream_local_internal::<C>(
            init,
            self.token.child_token(),
            &self.tracker,
            self.original_runtime.clone(),
        )
    }

    pub fn track<F>(&self, future: F)
    where
        F: Future<Output = ()> + Send + 'static,
    {
        self.tracker.spawn(future);
    }

    pub fn track_main<F>(&self, future: F)
    where
        F: Future<Output = ()> + Send + 'static,
    {
        self.tracker.spawn_on(future, &self.original_runtime);
    }
}

pub fn spawn<A: Actor>(token: CancellationToken, init: A::Init) -> Handle<A> {
    let tracker = TaskTracker::new();

    spawn_internal::<A>(token, &tracker, init, RuntimeHandle::current())
}

fn spawn_internal<A: Actor>(
    token: CancellationToken,
    tracker: &TaskTracker,
    init: A::Init,
    original_runtime: RuntimeHandle,
) -> Handle<A> {
    let (tx, mut rx) = mpsc::channel(A::channel_size());
    let handle = Handle::new(tx);

    let mut ctx = Context {
        handle: handle.clone(),
        token,
        tracker: tracker.clone(),
        original_runtime,
    };

    tracker.spawn(async move {
        let mut actor = A::init(init, &mut ctx).await;

        loop {
            tokio::select! {
                biased;
                _ = ctx.token.cancelled() => break,
                msg = rx.recv() => {
                    match msg {
                        Some(msg) => actor.handle_message(msg, &mut ctx).await,
                        None => break,
                    }
                }
            }
        }

        ctx.token.cancel();
        ctx.tracker.close();
        ctx.tracker.wait().await;

        actor.on_stop(&mut ctx).await;
    });

    handle
}

pub(crate) fn spawn_stream_internal<A: StreamActor>(
    init: A::Init,
    token: CancellationToken,
    tracker: &TaskTracker,
    original_runtime: RuntimeHandle,
) -> Handle<A> {
    let (tx, mut rx) = mpsc::channel(A::channel_size());
    let handle = Handle::new(tx);

    let mut ctx = Context {
        handle: handle.clone(),
        token,
        tracker: tracker.clone(),
        original_runtime,
    };

    tracker.spawn(async move {
        let mut actor = A::init(init, &mut ctx).await;

        pin! {
            let stream = actor.create_stream(&mut ctx).await;
        }

        loop {
            tokio::select! {
                biased;
                _ = ctx.token.cancelled() => break,
                item = stream.next() => {
                    match item {
                        Some(event) => actor.handle_event(event, &mut ctx).await,
                        None => break,
                    }
                }
                msg = rx.recv() => {
                    match msg {
                        Some(msg) => actor.handle_message(msg, &mut ctx).await,
                        None => break,
                    }
                }
            }
        }

        ctx.token.cancel();
        ctx.tracker.close();
        ctx.tracker.wait().await;

        actor.on_stop(&mut ctx).await;
    });
    handle
}

fn spawn_stream_local_internal<A: LocalStreamActor>(
    init: A::Init,
    token: CancellationToken,
    parent_tracker: &TaskTracker,
    original_runtime: RuntimeHandle,
) -> Handle<A> {
    let (tx, mut rx) = mpsc::channel(A::channel_size());
    let handle = Handle::new(tx);
    let ctx_handle = handle.clone();

    let (done_tx, done_rx) = oneshot::channel::<()>();

    std::thread::spawn(move || {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("failed to create local actor runtime");

        let local = LocalSet::new();

        local.spawn_local(async move {
            let tracker = TaskTracker::new();
            let mut ctx = Context {
                handle: ctx_handle,
                token: token,
                tracker,
                original_runtime,
            };

            // run_local_stream_actor::<A>(init, rx, &mut ctx).await;

            let mut actor = A::init(init, &mut ctx).await;

            pin! {
                 let stream = actor.create_stream(&mut ctx).await;
            };

            loop {
                tokio::select! {
                    biased;
                    _ = ctx.token.cancelled() => break,
                    item = stream.next() => {
                        match item {
                            Some(event) => actor.handle_event(event,&mut ctx).await,
                            None => break,
                        }
                    }
                    msg = rx.recv() => {
                        match msg {
                            Some(msg) => actor.handle_message(msg, &mut ctx).await,
                            None => break,
                        }
                    }
                }
            }

            ctx.token.cancel();
            ctx.tracker.close();

            actor.on_stop(&mut ctx).await;

            let _ = done_tx.send(());
        });

        rt.block_on(local);
    });

    parent_tracker.spawn(async move {
        let _ = done_rx.await;
    });

    handle
}
