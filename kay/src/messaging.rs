use actor_system::{ID, Known, World};
use compact::Compact;

pub trait Recipient<M: Message> {
    fn receive(&mut self, message: &M, world: &mut World);
}

pub trait Message : Compact + Known {}

pub struct MessagePacket<M: Message> {
    pub recipient_id: ID,
    pub message: M
}

impl<M: Message> Compact for MessagePacket<M> {
    fn is_still_compact(&self) -> bool {self.message.is_still_compact()}
    fn dynamic_size_bytes(&self) -> usize {self.message.dynamic_size_bytes()}
    unsafe fn compact_from(&mut self, source: &Self, new_dynamic_part: *mut u8) {
        self.recipient_id = source.recipient_id;
        self.message.compact_from(&source.message, new_dynamic_part);
    }
}