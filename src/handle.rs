use tokio::sync::{mpsc, oneshot};

use super::mailbox::HasMailbox;

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

    pub async fn cast(&self, msg: A::Message) -> Result<(), ActorDead<A::Message>> {
        self.sender.send(msg).await.map_err(|e| ActorDead(e.0))
    }

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

#[derive(Debug)]
pub struct ActorDead<M>(pub M);

impl<M> std::fmt::Display for ActorDead<M> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "actor is no longer running")
    }
}

impl<M: std::fmt::Debug> std::error::Error for ActorDead<M> {}
