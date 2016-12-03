use super::actor_system::{ID};
use super::compact::Compact;
use ::std::mem::size_of;

pub enum Fate{
    Live,
    Die
}

pub trait Recipient<M: Message> {
    fn receive_packet(&mut self, packet: &Packet<M>) -> Fate {
        self.receive(&packet.message)
    }
    fn receive(&mut self, _message: &M) -> Fate {unimplemented!()}
}

pub trait StorageAware : Sized {
    fn typical_size() -> usize {
        // TODO: create versions of containers for 0 size messages & actors
        let size = size_of::<Self>();
        if size == 0 {1} else {size}
    }
}
impl <T> StorageAware for T{}

pub trait Message : Compact + StorageAware + 'static {}
impl <T: Compact + 'static > Message for T{}
pub trait Actor : Compact + StorageAware + 'static {
    fn id(&self) -> ID;
    unsafe fn set_id(&mut self, id: ID);
}

pub trait Individual : 'static {
    fn id() -> ID where Self: Sized{
        unsafe{
            (*super::actor_system::THE_SYSTEM).individual_id::<Self>()
        }
    }
}

#[derive(Clone)]
pub struct Packet<M: Message> {
    pub recipient_id: ID,
    pub message: M
}

impl<M: Message> Compact for Packet<M> {
    fn is_still_compact(&self) -> bool {self.message.is_still_compact()}
    fn dynamic_size_bytes(&self) -> usize {self.message.dynamic_size_bytes()}
    unsafe fn compact_from(&mut self, source: &Self, new_dynamic_part: *mut u8) {
        self.recipient_id = source.recipient_id;
        self.message.compact_from(&source.message, new_dynamic_part);
    }
    unsafe fn decompact(&self) -> Packet<M> {
        Packet{
            recipient_id: self.recipient_id,
            message: self.message.decompact()
        }
    }
}