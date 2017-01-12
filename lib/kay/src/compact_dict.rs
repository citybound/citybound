use super::allocators::{Allocator, DefaultHeap};
use super::compact::Compact;
use super::compact_vec::CompactVec;

/// A dictionary which can be compacted. Will usually contain the data in the dynamic part of the
/// compacted data structure, but if the size will overflow the dynamic part, it is moved to the
/// heap instead.
/// Implemented with 2 vectors, one for the key and one for the data, with linear search performed
/// to do lookups
pub struct CompactDict<K: Copy, V: Compact + Clone, A: Allocator = DefaultHeap> {
    keys: CompactVec<K, A>,
    values: CompactVec<V, A>,
}

impl<K: Eq + Copy, V: Compact + Clone, A: Allocator> CompactDict<K, V, A> {
    /// Create new, empty dict
    pub fn new() -> Self {
        CompactDict {
            keys: CompactVec::new(),
            values: CompactVec::new(),
        }
    }

    /// Amount of entries in the dict
    pub fn len(&self) -> usize {
        self.keys.len()
    }

    /// Is the dict empty
    pub fn is_empty(&self) -> bool {
        self.keys.is_empty()
    }

    /// Lookup the item in the dict
    pub fn get(&self, query: K) -> Option<&V> {
        for i in 0..self.keys.len() {
            if self.keys[i] == query {
                return Some(&self.values[i]);
            }
        }
        None
    }

    /// Lookup the item in the dict, but also swaps the item with the first element
    /// so a repeated lookup for that item will be faster
    pub fn get_mru(&mut self, query: K) -> Option<&V> {
        for i in 0..self.keys.len() {
            if self.keys[i] == query {
                self.keys.swap(0, i);
                self.values.swap(0, i);
                return Some(&self.values[0]);
            }
        }
        None
    }

    /// Lookup the item in the dict, but also swaps the item with the previous item
    /// so it makes its way to the beginning of the vec upon repeated lookup
    pub fn get_mfu(&mut self, query: K) -> Option<&V> {
        for i in 0..self.keys.len() {
            if self.keys[i] == query {
                if i > 0 {
                    self.keys.swap(i - 1, i);
                    self.values.swap(i - 1, i);
                    return Some(&self.values[i - 1]);
                } else {
                    return Some(&self.values[0]);
                }
            }
        }
        None
    }

    /// Checks if dict contains key
    pub fn contains_key(&self, query: K) -> bool {
        self.keys.contains(&query)
    }

    /// Insert new value at key
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

    /// Remove value at key
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

    /// get iterator over keys
    pub fn keys(&self) -> ::std::slice::Iter<K> {
        self.keys.iter()
    }

    /// Get iterator over values
    pub fn values(&self) -> ::std::slice::Iter<V> {
        self.values.iter()
    }

    /// Get mutable iterator over values
    pub fn values_mut(&mut self) -> ::std::slice::IterMut<V> {
        self.values.iter_mut()
    }

    /// Get iterator over key-value pairs
    #[allow(needless_lifetimes)]
    pub fn pairs<'a>(&'a self) -> impl Iterator<Item = (&'a K, &'a V)> + Clone + 'a {
        self.keys().zip(self.values())
    }
}

impl<K: Eq + Copy, I: Compact, A1: Allocator, A2: Allocator> CompactDict<K, CompactVec<I, A1>, A2> {
    /// Push a value to a CompactVec at the key
    pub fn push_at(&mut self, query: K, item: I) {
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

    /// Get iterator of the CompactVec at the key
    #[allow(needless_lifetimes)]
    pub fn get_iter<'a>(&'a self, query: K) -> impl Iterator<Item = &'a I> + 'a {
        self.get(query).into_iter().flat_map(|vec_in_option| vec_in_option.iter())
    }

    /// Get iterator of the CompactVec at the key, and then remove the item
    #[allow(needless_lifetimes)]
    pub fn remove_iter<'a>(&'a mut self, query: K) -> impl Iterator<Item = I> + 'a {
        self.remove(query).into_iter().flat_map(|vec_in_option| vec_in_option.into_iter())
    }
}

impl<K: Copy, V: Compact + Clone, A: Allocator> Compact for CompactDict<K, V, A> {
    fn is_still_compact(&self) -> bool {
        self.keys.is_still_compact() && self.values.is_still_compact()
    }

    fn dynamic_size_bytes(&self) -> usize {
        self.keys.dynamic_size_bytes() + self.values.dynamic_size_bytes()
    }

    unsafe fn compact_from(&mut self, source: &Self, new_dynamic_part: *mut u8) {
        self.keys.compact_from(&source.keys, new_dynamic_part);
        self.values.compact_from(&source.values,
                                 new_dynamic_part.offset(self.keys.dynamic_size_bytes() as isize));
    }

    unsafe fn decompact(&self) -> CompactDict<K, V, A> {
        CompactDict {
            keys: self.keys.decompact(),
            values: self.values.decompact(),
        }
    }
}

impl<K: Copy, V: Compact + Clone, A: Allocator> Clone for CompactDict<K, V, A> {
    fn clone(&self) -> Self {
        CompactDict {
            keys: self.keys.clone(),
            values: self.values.clone(),
        }
    }
}

impl<K: Copy + Eq, V: Compact + Clone, A: Allocator> Default for CompactDict<K, V, A> {
    fn default() -> Self {
        CompactDict::new()
    }
}

impl <K: Copy + Eq, V: Compact + Clone, A: Allocator> ::std::iter::FromIterator<(K, V)> for CompactDict<K, V, A> {
    fn from_iter<T: IntoIterator<Item=(K, V)>>(iter: T) -> Self {
        let mut dict = Self::new();
        for (key, value) in iter {
            dict.insert(key, value);
        }
        dict
    }
}

impl <K: Copy + Eq, V: Compact + Clone, A: Allocator> ::std::iter::Extend<(K, V)> for CompactDict<K, V, A> {
    fn extend<T: IntoIterator<Item=(K, V)>>(&mut self, iter: T) {
        for (key, value) in iter {
            self.insert(key, value);
        }
    }
}
