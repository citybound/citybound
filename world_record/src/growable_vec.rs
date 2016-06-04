extern crate std;
use std::path::PathBuf;

use growable_buffer::GrowableBuffer;

struct GrowableVecHeader {
    len: usize,
}

impl Default for GrowableVecHeader {
    fn default() -> GrowableVecHeader {
        return GrowableVecHeader{len: 0};
    }
}

pub struct GrowableVec<Item> {
    buffer: GrowableBuffer<GrowableVecHeader, Item>,
}

impl<Item> GrowableVec<Item> {
    pub fn new(path: PathBuf) -> GrowableVec<Item> {
        return GrowableVec { buffer: GrowableBuffer::new(path) };
    }

    pub fn len(&self) -> usize {
        return self.buffer.header.len;
    }

    pub fn push(&mut self, value: Item) {
        let new_len = self.len() + 1;
        self.buffer.require_cap(new_len);
        self.buffer.header.len += 1;
        let new_last_index = self.len() - 1;
        self[new_last_index] = value;
    }

    pub fn pop(&mut self) -> Option<Item> {
        if self.len() == 0 {
            return None;
        } else {
            unsafe {
                let new_len = self.len() - 1;
                self.buffer.header.len -= new_len;
                let val = std::ptr::read(self.get_unchecked(self.len()));
                self.buffer.require_cap(new_len);
                return Some(val);
            }
        }
    }

    pub fn swap_remove(&mut self, index: usize) -> Item {
        let last_index = self.len() - 1;
        self.swap(index, last_index);
        return self.pop().unwrap();
    }
}

impl<Item> std::ops::Deref for GrowableVec<Item> {
    type Target = [Item];

    fn deref(&self) -> &[Item] {
        return &self.buffer.items[..self.len()];
    }
}

impl<Item> std::ops::DerefMut for GrowableVec<Item> {
    fn deref_mut(&mut self) -> &mut [Item] {
        let len = self.len();
        return &mut self.buffer.items[..len];
    }
}
