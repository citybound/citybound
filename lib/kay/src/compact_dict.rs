use super::allocators::{Allocator, DefaultHeap};
use super::compact::Compact;
use super::compact_vec::CompactVec;

pub struct CompactDict <K: Copy, V: Compact + Clone, A: Allocator = DefaultHeap> {
    keys: CompactVec<K, A>,
    values: CompactVec<V, A>
}

impl <K: Eq + Copy, V: Compact + Clone + ::std::fmt::Debug, A: Allocator> CompactDict<K, V, A> {
    pub fn new() -> Self {
        CompactDict{
            keys: CompactVec::new(),
            values: CompactVec::new()
        }
    }

    pub fn get(&self, query: K) -> Option<&V> {
        for i in 0..self.keys.len() {
            if self.keys[i] == query {
                return Some(&self.values[i])
            }
        }
        None
    }

    pub fn insert(&mut self, query: K, new_value: V) -> Option<V> {
        for i in 0..self.keys.len() {
            if self.keys[i] == query {
                let old_val = self.values[i].clone();
                self.values[i] = new_value;
                return Some(old_val);
            }
        }
        self.keys.push(query);
        self.values.push(new_value);
        None
    }

    pub fn remove(&mut self, query: K) -> Option<V> {
        for i in 0..self.keys.len() {
            if self.keys[i] == query {
                let old_val = self.values[i].clone();
                self.keys.remove(i);
                self.values.remove(i);
                return Some(old_val);
            }
        }
        None
    }

    pub fn keys(&self) -> ::std::slice::Iter<K> {
        self.keys.iter()
    }

    pub fn values(&self) -> ::std::slice::Iter<V> {
        self.values.iter()
    }
}

impl <K: Eq + Copy, I: Compact + ::std::fmt::Debug, A: Allocator> CompactDict<K, CompactVec<I>, A> {
    pub fn push_or_create_at(&mut self, query: K, item: I) {
        for i in 0..self.keys.len() {
            if self.keys[i] == query {
                self.values[i].push(item);
                return;
            }
        }
        self.keys.push(query);
        let mut vec = CompactVec::new();
        vec.push(item);
        self.values.push(vec);
    }
} 

impl <K: Copy, V: Compact + Clone, A: Allocator> Compact for CompactDict<K, V, A> {
    fn is_still_compact(&self) -> bool {
        self.keys.is_still_compact() && self.values.is_still_compact()
    }

    fn dynamic_size_bytes(&self) -> usize {
        self.keys.dynamic_size_bytes() + self.values.dynamic_size_bytes()
    }

    unsafe fn compact_from(&mut self, source: &Self, new_dynamic_part: *mut u8) {
        self.keys.compact_from(&source.keys, new_dynamic_part);
        self.values.compact_from(&source.values, new_dynamic_part.offset(self.keys.dynamic_size_bytes() as isize));
    }

    unsafe fn decompact(&self) -> CompactDict<K, V, A> {
        CompactDict{
            keys: self.keys.decompact(),
            values: self.values.decompact()
        }
    }
}

impl <K: Copy, V: Compact + Clone, A: Allocator> Clone for CompactDict<K, V, A> {
    fn clone(&self) -> Self {
        CompactDict{
            keys: self.keys.clone(),
            values: self.values.clone()
        }
    }
}