use super::id::ID;
use super::compact::Compact;

/// Return type of message handling functions, signifying if
/// an `Actor`/`SubActor` should live on after receiving a certain message type.
///
/// Note: so far only has an effect on `SubActor`s in `Swarm`s
pub enum Fate {
    /// Means: the `Actor`/`SubActor` should live on
    Live,
    /// Means: the `Actor`/`SubActor` should be stopped, its state can be deallocated
    Die,
}

/// Trait that a datastructure must implement in order
/// to be sent and received as a message.
///
/// Automatically implemented for everything that is [`Compact`](../../compact)
pub trait Message: Compact + 'static {}
impl<T: Compact + 'static> Message for T {}

/// Combination of a message and its destination recipient id
#[derive(Compact, Clone)]
pub struct Packet<M: Message> {
    /// ID of the `Actor`/`SubActor` that should receive this message
    pub recipient_id: ID,
    /// The message itself
    pub message: M,
}
