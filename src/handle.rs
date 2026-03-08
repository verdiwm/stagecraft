use tokio::sync::{mpsc, oneshot};

use super::mailbox::HasMailbox;

/// A cloneable handle to a running actor.
///
/// `Handle<A>` wraps the actor's inbox sender. Clone it freely to share access
/// across tasks. When all handles are dropped the actor's inbox closes, which
/// causes the message loop to exit.
pub struct Handle<A: HasMailbox> {
    sender: mpsc::Sender<A::Message>,
}

impl<A: HasMailbox> Clone for Handle<A> {
    fn clone(&self) -> Self {
        Self {
            sender: self.sender.clone(),
        }
    }
}

impl<A: HasMailbox> std::fmt::Debug for Handle<A> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Handle").finish_non_exhaustive()
    }
}

impl<A: HasMailbox> Handle<A> {
    pub(crate) fn new(sender: mpsc::Sender<A::Message>) -> Self {
        Self { sender }
    }

    /// Send a message without waiting for a reply.
    ///
    /// Returns [`Err`]`(`[`ActorDead`]`)` if the actor is no longer running.
    pub async fn cast(&self, msg: A::Message) -> Result<(), ActorDead<A::Message>> {
        self.sender.send(msg).await.map_err(|e| ActorDead(e.0))
    }

    /// Send a message and wait for a reply.
    ///
    /// `make_msg` receives a [`oneshot::Sender`] and must return the message to send.
    /// The actor is responsible for sending a value through that channel.
    ///
    /// Returns [`Err`]`(`[`ActorDead`]`)` if the actor is no longer running or the
    /// response channel is dropped.
    ///
    /// [`oneshot::Sender`]: tokio::sync::oneshot::Sender
    pub async fn call<R, F>(&self, make_msg: F) -> Result<R, ActorDead<()>>
    where
        F: FnOnce(oneshot::Sender<R>) -> A::Message,
    {
        let (tx, rx) = oneshot::channel();
        self.sender
            .send(make_msg(tx))
            .await
            .map_err(|_| ActorDead(()))?;
        rx.await.map_err(|_| ActorDead(()))
    }
}

/// Error returned when a message cannot be delivered because the actor is no longer running.
///
/// The inner value `M` is:
/// - the original message for [`Handle::cast`] (so the caller can recover it), or
/// - `()` for [`Handle::call`] operations.
#[derive(Debug)]
pub struct ActorDead<M>(pub M);

impl<M> std::fmt::Display for ActorDead<M> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "actor is no longer running")
    }
}

impl<M: std::fmt::Debug> std::error::Error for ActorDead<M> {}
