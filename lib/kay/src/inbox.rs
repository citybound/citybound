use super::compact::Compact;
use super::messaging::{Packet, Message};
use super::chunked::{MemChunker, ChunkedQueue};
use super::type_registry::{ShortTypeId, TypeRegistry};

pub struct Inbox {
    queue: ChunkedQueue,
}

const CHUNK_SIZE: usize = 4096 * 4096 * 4; // 64MB

impl Inbox {
    pub fn new() -> Self {
        let chunker = MemChunker::from_settings("", CHUNK_SIZE);
        Inbox { queue: ChunkedQueue::new(chunker) }
    }

    pub fn put<M: Message>(&mut self, packet: Packet<M>, message_registry: &TypeRegistry) {
        let packet_size = packet.total_size_bytes();
        let total_size = ::std::mem::size_of::<ShortTypeId>() + packet_size;

        unsafe {

            // "Allocate" the space in the queue
            let queue_ptr = self.queue.enqueue(total_size);

            // Write message type
            *(queue_ptr as *mut ShortTypeId) = message_registry.get::<M>();

            let payload_ptr = queue_ptr.offset(::std::mem::size_of::<ShortTypeId>() as isize);

            // Get the address of the location in the queue
            let packet_in_queue = &mut *(payload_ptr as *mut Packet<M>);

            // Write the packet into the queue
            ::std::ptr::write_unaligned(payload_ptr as *mut Packet<M>, packet);
            packet_in_queue.compact_behind();
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
    queue: &'a mut ChunkedQueue,
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
            unsafe {
                let ptr = self.queue.dequeue().expect(
                    "should have something left for sure",
                );
                let message_type = *(ptr as *mut ShortTypeId);
                let payload_ptr = ptr.offset(::std::mem::size_of::<ShortTypeId>() as isize);
                self.n_messages_to_read -= 1;
                Some(DispatchablePacket {
                    message_type: message_type,
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

impl Default for Inbox {
    fn default() -> Self {
        Self::new()
    }
}
