use super::allocators::{Allocator, DefaultHeap};
use super::compact::Compact;
use super::compact_vec::CompactVec;

pub struct CompactDict <K, V, A: Allocator = DefaultHeap> {
    pairs: CompactVec<(K, V), A>
}

impl <K: Eq + Copy, V: Copy, A: Allocator> CompactDict<K, V, A> {
    pub fn new() -> Self {
        CompactDict{
            pairs: CompactVec::new()
        }
    }

    pub fn get(&self, query: K) -> Option<&V> {
        for &(ref key, ref value) in self.pairs.iter() {
            if query == *key {return Some(&value)};
        }
        None
    }


    pub fn insert(&mut self, query: K, new_value: V) -> Option<V> {
        for &mut (ref mut key, ref mut value) in &mut self.pairs.iter_mut() {
            if query == *key {
                let old_val = value.clone();
                *value = new_value;
                return Some(old_val);
            };
        }
        self.pairs.push((query, new_value));
        None
    }

    fn get_key(pair: &(K, V)) -> K {pair.0}

    pub fn keys<'a>(&'a self) -> ::std::iter::Map<::std::slice::Iter<'a, (K, V)>, fn(&(K, V)) -> K > {
        return self.pairs.iter().map(Self::get_key);
    }
}

impl <K: Copy, V: Copy, A: Allocator> Compact for CompactDict<K, V, A> {
    fn is_still_compact(&self) -> bool {
        self.pairs.is_still_compact()
    }

    fn dynamic_size_bytes(&self) -> usize {
        self.pairs.dynamic_size_bytes()
    }

    unsafe fn compact_from(&mut self, source: &Self, new_dynamic_part: *mut u8) {
        self.pairs.compact_from(&source.pairs, new_dynamic_part);
    }
}