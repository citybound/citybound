use super::compact::{Compact};
use super::chunked::{MemChunker, MultiSized, SizedChunkedQueue};
use ::std::marker::PhantomData;
use super::messaging::{Packet, Message};

pub struct Inbox<M: Message> {
    queues: MultiSized<SizedChunkedQueue>,
    message_marker: PhantomData<[M]>
}

const CHUNK_SIZE : usize = 4096;

impl <M: Message> Inbox<M> {
    pub fn new() -> Self {
        let chunker = MemChunker::new("", CHUNK_SIZE);
        Inbox {
            queues: MultiSized::new(chunker, M::typical_size()),
            message_marker: PhantomData
        }
    }

    pub fn put(&mut self, package: Packet<M>) {
        let required_size = package.total_size_bytes();
        unsafe {
            let raw_ptr = self.queues.sized_for_mut(required_size).enqueue();
            let message_in_slot = &mut *(raw_ptr as *mut Packet<M>);
            message_in_slot.compact_behind_from(&package);
        }
    }

    pub fn empty(&mut self) -> InboxIterator<M> {
        // one higher than last index, first next() will init messages left
        let start_queue_index = self.queues.collections.len();
        InboxIterator {
            queues: &mut self.queues.collections,
            current_sized_queue_index: start_queue_index,
            messages_in_sized_queue_left: 0,
            message_marker: PhantomData
        }
    }
}

// once created, reads all messages that are there roughly at the point of creation
// that means that once it terminates there might already be new messages in the inbox
pub struct InboxIterator<'a, M: Message> where M: 'a {
    queues: &'a mut Vec<SizedChunkedQueue>,
    current_sized_queue_index: usize,
    messages_in_sized_queue_left: usize,
    message_marker: PhantomData<[M]>
}

impl<'a, M: Message> Iterator for InboxIterator<'a, M> {
    type Item = &'a Packet<M>;

    fn next(&mut self) -> Option<&'a Packet<M>> {
        if self.messages_in_sized_queue_left == 0 {
            if self.current_sized_queue_index == 0 {
                None
            } else {
                self.current_sized_queue_index -= 1;
                {
                    let next_queue = &self.queues[self.current_sized_queue_index];
                    self.messages_in_sized_queue_left = *next_queue.write_index - *next_queue.read_index;
                }
                self.next()
            }
        } else {
            unsafe {
                let raw_ptr = self.queues[self.current_sized_queue_index].dequeue().unwrap();
                let message_ref = &*(raw_ptr as *const Packet<M>);
                self.messages_in_sized_queue_left -= 1;
                Some(message_ref)
            }
        }
    }
}