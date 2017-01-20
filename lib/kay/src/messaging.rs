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

/// Trait with which message handling for `Actor`/`SubActor`s is implemented
pub trait Recipient<M: Message> {
    /// Let's an actor mutate its state when it gets a message,
    /// send messages to other actors and determine its new `Fate`
    fn receive(&mut self, _message: &M) -> Fate {
        unimplemented!()
    }
    /// Like `receive`, but allows access to `packet.recipient_id`
    /// that the packet/message was sent to. This is used by `Swarm`
    /// to dispatch the message to the correct `SubActor`.
    ///
    /// The default implementation just calls `receive` with `packet.message`
    fn receive_packet(&mut self, packet: &Packet<M>) -> Fate {
        self.receive(&packet.message)
    }
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
    pub recipient_id: Option<ID>,
    /// The message itself
    pub message: M,
}
