use super::compact::{Compact};
use super::chunked::{Chunker, MultiSized, SizedChunkedQueue};
use ::std::marker::PhantomData;
use ::std::mem::transmute;
use super::messaging::{MessagePacket, Message};

pub struct Inbox<M: Message> {
    queues: MultiSized<SizedChunkedQueue>,
    message_marker: PhantomData<[M]>
}

impl <M: Message> Inbox<M> {
    pub fn new(chunker: Box<Chunker>, base_size: usize) -> Self {
        Inbox {
            queues: MultiSized::new(chunker, base_size),
            message_marker: PhantomData
        }
    }

    pub fn put(&mut self, package: MessagePacket<M>) {
        let required_size = package.total_size_bytes();
        unsafe {
            let raw_ptr = self.queues.sized_for_mut(required_size).enqueue();
            let message_in_slot : &mut MessagePacket<M> = transmute(raw_ptr);
            message_in_slot.compact_behind_from(&package);
        }
    }

    pub fn empty<'a>(&'a mut self) -> InboxIterator<'a, M> {
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
    type Item = &'a MessagePacket<M>;

    fn next(&mut self) -> Option<&'a MessagePacket<M>> {
        if self.messages_in_sized_queue_left == 0 {
            if self.current_sized_queue_index == 0 {
                None
            } else {
                self.current_sized_queue_index -= 1;
                {
                    let ref next_queue = self.queues[self.current_sized_queue_index];
                    self.messages_in_sized_queue_left = *next_queue.write_index - *next_queue.read_index;
                }
                self.next()
            }
        } else {
            unsafe {
                let raw_ptr = self.queues[self.current_sized_queue_index].dequeue().unwrap();
                let message_ref : &'a MessagePacket<M> = transmute(raw_ptr);
                self.messages_in_sized_queue_left -= 1;
                Some(message_ref)
            }
        }
    }
}