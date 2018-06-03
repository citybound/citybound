use super::compact::Compact;
use super::chunky;
use super::messaging::{Packet, Message};
use super::type_registry::{ShortTypeId, TypeRegistry};

pub struct Inbox {
    queue: chunky::Queue<chunky::HeapHandler>,
}

const CHUNK_SIZE: usize = 1024 * 1024 * 4; // 64MB

impl Inbox {
    pub fn new(ident: &chunky::Ident) -> Self {
        Inbox {
            queue: chunky::Queue::new(ident, CHUNK_SIZE),
        }
    }

    pub fn put<M: Message>(&mut self, mut packet: Packet<M>, message_registry: &TypeRegistry) {
        let packet_size = packet.total_size_bytes();
        let total_size = ::std::mem::size_of::<ShortTypeId>() + packet_size;

        #[cfg_attr(feature = "cargo-clippy", allow(cast_ptr_alignment))]
        unsafe {
            // "Allocate" the space in the queue
            let queue_ptr = self.queue.enqueue(total_size);

            // Write message type
            *(queue_ptr as *mut ShortTypeId) = message_registry.get::<M>();

            let payload_ptr = queue_ptr.offset(::std::mem::size_of::<ShortTypeId>() as isize);

            // Write the packet into the queue
            Compact::compact_behind(&mut packet, payload_ptr as *mut Packet<M>);
            ::std::mem::forget(packet);
        }
    }

    pub fn put_raw(&mut self, buf: &[u8]) {
        unsafe {
            let queue_ptr = self.queue.enqueue(buf.len());

            ::std::ptr::copy_nonoverlapping(&buf[0], queue_ptr, buf.len())
        }
    }

    pub fn empty(&mut self) -> InboxIterator {
        InboxIterator {
            n_messages_to_read: self.queue.len(),
            queue: &mut self.queue,
        }
    }
}

pub struct InboxIterator<'a> {
    queue: &'a mut chunky::Queue<chunky::HeapHandler>,
    n_messages_to_read: usize,
}

pub struct DispatchablePacket {
    pub message_type: super::type_registry::ShortTypeId,
    pub packet_ptr: *const (),
}

impl<'a> Iterator for InboxIterator<'a> {
    type Item = DispatchablePacket;

    fn next(&mut self) -> Option<DispatchablePacket> {
        if self.n_messages_to_read == 0 {
            None
        } else {
            #[cfg_attr(feature = "cargo-clippy", allow(cast_ptr_alignment))]
            unsafe {
                let ptr = self
                    .queue
                    .dequeue()
                    .expect("should have something left for sure");
                let message_type = *(ptr as *mut ShortTypeId);
                let payload_ptr = ptr.offset(::std::mem::size_of::<ShortTypeId>() as isize);
                self.n_messages_to_read -= 1;
                Some(DispatchablePacket {
                    message_type,
                    packet_ptr: payload_ptr as *const (),
                })
            }
        }
    }
}

impl<'a> Drop for InboxIterator<'a> {
    fn drop(&mut self) {
        unsafe { self.queue.drop_old_chunks() };
    }
}
