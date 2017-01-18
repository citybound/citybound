use super::id::ID;
use super::compact::Compact;

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

pub trait Message: Compact + 'static {}
impl<T: Compact + 'static> Message for T {}

#[derive(Compact, Clone)]
pub struct Packet<M: Message> {
    pub recipient_id: Option<ID>,
    pub message: M,
}
