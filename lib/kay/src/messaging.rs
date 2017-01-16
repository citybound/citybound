use super::id::ID;
use super::compact::Compact;
use ::std::mem::size_of;

pub enum Fate {
    Live,
    Die,
}

pub trait Recipient<M: Message> {
    /// Default implementation to ignore the address
    /// Overridden by `swarm` to to provide per instance addressing
    fn receive_packet(&mut self, packet: &Packet<M>) -> Fate {
        self.receive(&packet.message)
    }
    fn receive(&mut self, _message: &M) -> Fate {
        unimplemented!()
    }
}

pub trait StorageAware: Sized {
    fn typical_size() -> usize {
        // TODO: create versions of containers for 0 size messages & actors
        let size = size_of::<Self>();
        if size == 0 { 1 } else { size }
    }
}
impl<T> StorageAware for T {}

pub trait Message: Compact + StorageAware + 'static {}
impl<T: Compact + 'static> Message for T {}
pub trait Actor: Compact + StorageAware + 'static {
    fn id(&self) -> ID;
    unsafe fn set_id(&mut self, id: ID);
}

pub trait Individual: 'static {
    fn id() -> ID
        where Self: Sized
    {
        unsafe { (*super::THE_SYSTEM).individual_id::<Self>() }
    }
}

#[derive(Compact, Clone)]
pub struct Packet<M: Message> {
    pub recipient_id: Option<ID>,
    pub message: M,
}
