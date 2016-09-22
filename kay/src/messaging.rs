use actor_system::{ID, Known, SystemServices};
use embedded::Embedded;

pub trait Recipient<M: Message> : Known {
    fn receive(&mut self, message: &M, system: &mut SystemServices);
}

pub trait Message : Embedded + Known {}

pub struct MessagePacket<M: Message> {
    pub recipient_id: ID,
    pub message: M
}

impl<M: Message> Embedded for MessagePacket<M> {
    fn is_still_embedded(&self) -> bool {self.message.is_still_embedded()}
    fn dynamic_size_bytes(&self) -> usize {self.message.dynamic_size_bytes()}
    unsafe fn embed_from(&mut self, source: &Self, new_dynamic_part: *mut u8) {
        self.recipient_id = source.recipient_id;
        self.message.embed_from(&source.message, new_dynamic_part);
    }
}