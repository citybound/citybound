use actor_system::{ID, Known, World};
use compact::Compact;

pub trait Recipient<M: Message> {
    fn receive(&mut self, message: &M, world: &mut World, self_id: ID);
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

#[macro_export]
macro_rules! message {
    ($name: path, $type_id: path) => (
        impl Message for $name {}
        impl Known for $name{fn type_id() -> usize {$type_id as usize}}
    )
}

#[macro_export]
macro_rules! recipient {
    ($name: path, (&mut $the_self: ident, $world: ident: &mut $World: ty, $self_id: ident: $ID: ty) {
        $($message_type: path: $message_pat: pat => $receive_impl: block),*
    }) => (
        $(
            impl Recipient<$message_type> for $name {
                #[allow(unused_variables)]
                fn receive (&mut $the_self, message: &$message_type, $world: &mut $World, $self_id: $ID) {
                    match message {
                        $message_pat => $receive_impl
                    }
                }
            }
        )*
    )
}