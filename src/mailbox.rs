/// Base trait for all actors.
///
/// Defines the actor's message type and inbox channel capacity. All actor traits
/// ([`Actor`], [`LocalActor`], [`StreamActor`], [`LocalStreamActor`]) require `HasMailbox`.
///
/// [`Actor`]: crate::Actor
/// [`LocalActor`]: crate::LocalActor
/// [`StreamActor`]: crate::StreamActor
/// [`LocalStreamActor`]: crate::LocalStreamActor
pub trait HasMailbox: Sized + 'static {
    /// The type of messages this actor accepts.
    type Message: Send + 'static;

    /// Inbox channel capacity. Defaults to `128`.
    fn channel_size() -> usize {
        128
    }
}
