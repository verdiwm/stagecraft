pub trait HasMailbox: Sized + 'static {
    type Message: Send + 'static;

    fn channel_size() -> usize {
        128
    }
}
