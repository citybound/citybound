use super::allocators::{Allocator, DefaultHeap};
use super::compact::Compact;
use super::compact_vec::CompactVec;
use super::compact_dict::CompactDict;
use super::compact_array::CompactArray;
use std::iter::Iterator;
use std::collections::hash_map::DefaultHasher;
use std::hash::Hasher;
use std::hash::Hash;
use std::marker::PhantomData;

#[derive(Clone)]
struct Entry<K: Copy + Hash + Eq, V: Compact + Clone> {
    key: K,
    hash: u64,
    value: V,
    used: bool,
}

impl<K: Copy + Hash + Eq, V: Compact + Clone> Compact for Entry<K, V> {
    fn is_still_compact(&self) -> bool {
        self.value.is_still_compact()
    }

    fn dynamic_size_bytes(&self) -> usize {
        self.value.dynamic_size_bytes()
    }

    unsafe fn compact_from(&mut self, source: &Self, new_dynamic_part: *mut u8) {
        self.value.compact_from(&source.value, new_dynamic_part);
    }

    unsafe fn decompact(&self) -> Entry<K, V> {
        Entry {
            key: self.key.clone(),
            value: self.value.decompact(),
            hash: self.hash,
            used: self.used
        }
    }
}
pub struct CompactHashMap<K: Copy + Eq + Hash, V: Compact + Clone, A: Allocator = DefaultHeap> {
    dict: CompactDict<K,V,A>,
    oa: OpenAddressingMap<K,V,A>,
}

struct OpenAddressingMap<K: Copy + Eq + Hash, V: Compact + Clone, A: Allocator = DefaultHeap> {
    size: usize,
    entries: CompactArray<Entry<K, V>, A>,
}

impl<K: Copy + Eq + Hash, V: Compact + Clone, A: Allocator> OpenAddressingMap<K, V, A> {
    pub fn with_capacity(l: usize) -> Self {
        OpenAddressingMap {
            entries: CompactArray::with_capacity(100),
            size: l,
        }
    }

    /// Amount of entries in the dictionary
    pub fn len(&self) -> usize {
        self.size
    }

    /// Is the dictionary empty?
    pub fn is_empty(&self) -> bool {
        self.size == 0
    }

    /// Look up the value for key `query`, if it exists
    pub fn get(&self, query: K) -> Option<&V> {
        self.get_inner(query).map(|e|{&e.value})
    }

    /// Does the dictionary contain a value for `query`?
    pub fn contains_key(&self, query: K) -> bool {
        self.get(query).map_or(false, |i|{true})
    }

    /// Insert new value at key `query` and return the previous value at that key, if any existed
    pub fn insert(&mut self, query: K, value: V) -> Option<V> {
        self.ensure_capacity();
        let len = self.entries.capacity();
        let hash = self.hash(query);
        let h = hash as usize;
        for i in 0..len {
            let index = (h + i*i) % len;
            let entry = &mut self.entries[index];
            if !entry.used {
                entry.key = query;
                entry.value = value;
                entry.used = true;
                entry.hash = hash;
                return None
            } else if entry.key == query {
                let old_val: V = entry.value.clone();
                entry.value = value;
                entry.hash = hash;
                return Some(old_val)
            }
        }
        panic!("should always have place");
    }

    /// Remove value at key `query` and return it, if it existed
    pub fn remove(&mut self, query: K) -> Option<V> {
         self.remove_inner(query)
    }

    /// Iterator over all keys in the dictionary
    pub fn keys(&self) -> ::std::slice::Iter<K> {
         unimplemented!();
    }

    /// Iterator over all values in the dictionary
    pub fn values(&self) -> ::std::slice::Iter<V> {
         unimplemented!();
    }

    /// Iterator over mutable references to all values in the dictionary
    pub fn values_mut(&mut self) -> ::std::slice::IterMut<V> {
         unimplemented!();
    }

    /// Iterator over all key-value pairs in the dictionary
    //pub fn pairs<'a>(&'a self) -> impl Iterator<Item = (&'a K, &'a V)> + Clone + 'a {
    //    unimplemented!();
    //}

    fn hash(&self, key: K) -> u64 {
        let mut hasher = DefaultHasher::new();
        key.hash(&mut hasher);
        hasher.finish()
    }

    fn get_inner(&self, query: K) -> Option<&Entry<K,V>> {
        let len = self.entries.capacity();
        let h = self.hash(query) as usize;
        for i in 0..len {
            let index = (h + i*i) % len;
            if self.entries[index].used && (self.entries[index].key == query) {
                return Some(&self.entries[index]);
            }
        }
        None
    }

    fn remove_inner(&mut self, query: K) -> Option<V> {
        let len = self.entries.capacity();
        let h = self.hash(query) as usize;
        for i in 0..len {
            let index = (h + i*i) % len;
            if self.entries[index].used && (self.entries[index].key == query) {
                self.entries[index].used = false;
                return Some(self.entries[index].value.clone());
            }
        }
        None
    }

    fn ensure_capacity(&mut self) {
        if 10 * self.size > 7 * self.entries.capacity() {
            let old_entries = self.entries.clone();
            self.entries = CompactArray::with_capacity(old_entries.capacity() * 2);
            for entry in old_entries {
                self.insert(entry.key, entry.value);
            }
        }
    }
}

impl<K: Copy + Eq + Hash, V: Compact + Clone, A: Allocator> CompactHashMap<K, V, A> {
    /// Create new, empty dictionary
    pub fn new() -> Self {
        CompactHashMap {
            dict: CompactDict::new(),
            oa: OpenAddressingMap::with_capacity(100),
        }
    }

    /// Amount of entries in the dictionary
    pub fn len(&self) -> usize {
        self.dict.len()
    }

    /// Is the dictionary empty?
    pub fn is_empty(&self) -> bool {
        self.dict.is_empty()
    }

    /// Look up the value for key `query`, if it exists
    pub fn get(&self, query: K) -> Option<&V> {
        if let Some(res) = self.oa.get(query) {
            return Some(res);
        }
        self.dict.get(query)
    }

    /// Lookup up the value for key `query`, if it exists, but also swap the entry
    /// to the beginning of the key/value vectors, so a repeated lookup for that item will be faster
    pub fn get_mru(&mut self, query: K) -> Option<&V> {
        if let Some(res) = self.oa.get(query) {
            return Some(res);
        }
        self.dict.get_mru(query)
    }

    /// Lookup up the value for key `query`, if it exists, but also swap the entry
    /// one index towards the beginning of the key/value vectors, so frequently repeated lookups
    /// for that item will be faster
    pub fn get_mfu(&mut self, query: K) -> Option<&V> {
        if let Some(res) = self.oa.get(query) {
            return Some(res);
        }
        self.dict.get_mfu(query)
    }

    /// Does the dictionary contain a value for `query`?
    pub fn contains_key(&self, query: K) -> bool {
        if let Some(res) = self.oa.get(query) {
            return true;
        }
        self.dict.contains_key(query)
    }

    /// Insert new value at key `query` and return the previous value at that key, if any existed
    pub fn insert(&mut self, query: K, new_value: V) -> Option<V> {
        self.dict.insert(query, new_value)
    }

    /// Remove value at key `query` and return it, if it existed
    pub fn remove(&mut self, query: K) -> Option<V> {
        self.dict.remove(query)
    }

    /// Iterator over all keys in the dictionary
    pub fn keys(&self) -> ::std::slice::Iter<K> {
        self.dict.keys()
    }

    /// Iterator over all values in the dictionary
    pub fn values(&self) -> ::std::slice::Iter<V> {
        self.dict.values()
    }

    /// Iterator over mutable references to all values in the dictionary
    pub fn values_mut(&mut self) -> ::std::slice::IterMut<V> {
        self.dict.values_mut()
    }

    /// Iterator over all key-value pairs in the dictionary
    #[allow(needless_lifetimes)]
    pub fn pairs<'a>(&'a self) -> impl Iterator<Item = (&'a K, &'a V)> + Clone + 'a {
        self.dict.pairs()
    }
}

impl<K: Copy + Eq + Hash, I: Compact, A1: Allocator, A2: Allocator>
    CompactHashMap<K, CompactVec<I, A1>, A2> {
    /// Push a value onto the `CompactVec` at the key `query`
    pub fn push_at(&mut self, query: K, item: I) {
        self.dict.push_at(query, item)
    }

    /// Iterator over the `CompactVec` at the key `query`
    #[allow(needless_lifetimes)]
    pub fn get_iter<'a>(&'a self, query: K) -> impl Iterator<Item = &'a I> + 'a {
        self.dict.get_iter(query)
    }

    /// Remove the `CompactVec` at the key `query` and iterate over its elements (if it existed)
    #[allow(needless_lifetimes)]
    pub fn remove_iter<'a>(&'a mut self, query: K) -> impl Iterator<Item = I> + 'a {
        self.dict.remove_iter(query)
    }
}


impl<K: Copy + Eq + Hash, V: Compact + Clone, A: Allocator> Compact for OpenAddressingMap<K, V, A> {
    fn is_still_compact(&self) -> bool {
        self.entries.is_still_compact()
    }

    fn dynamic_size_bytes(&self) -> usize {
        self.entries.dynamic_size_bytes()
    }

    unsafe fn compact_from(&mut self, source: &Self, new_dynamic_part: *mut u8) {
        self.entries.compact_from(
            &source.entries,
            new_dynamic_part,
        );
        self.size.compact_from(
            &source.size,
            new_dynamic_part.offset(
                (self.entries.dynamic_size_bytes()) as isize,
            ),
        )
    }

    unsafe fn decompact(&self) -> OpenAddressingMap<K, V, A> {
        OpenAddressingMap {
            entries: self.entries.decompact(),
            size: self.size,
        }
    }
}

impl<K: Copy + Eq + Hash, V: Compact + Clone, A: Allocator> Compact for CompactHashMap<K, V, A> {
    fn is_still_compact(&self) -> bool {
        self.dict.is_still_compact() &&
            self.oa.is_still_compact()
    }

    fn dynamic_size_bytes(&self) -> usize {
        self.dict.dynamic_size_bytes() +
            self.oa.dynamic_size_bytes()
    }

    unsafe fn compact_from(&mut self, source: &Self, new_dynamic_part: *mut u8) {
        self.dict.compact_from(
            &source.dict,
            new_dynamic_part,
        );
        self.oa.compact_from(
            &source.oa,
            new_dynamic_part.offset(
                (self.dict.dynamic_size_bytes()) as isize,
            ),
        )
    }

    unsafe fn decompact(&self) -> CompactHashMap<K, V, A> {
        CompactHashMap {
            dict: self.dict.decompact(),
            oa: self.oa.decompact(),
        }
    }
}

impl<K: Copy + Eq + Hash, V: Compact + Clone, A: Allocator> Clone for OpenAddressingMap<K, V, A> {
    fn clone(&self) -> Self {
        OpenAddressingMap {
            entries: self.entries.clone(),
            size: self.size,
        }
    }
}

impl<K: Copy + Eq + Hash, V: Compact + Clone, A: Allocator> Clone for CompactHashMap<K, V, A> {
    fn clone(&self) -> Self {
        CompactHashMap {
            dict: self.dict.clone(),
            oa: self.oa.clone(),
        }
    }
}

impl<K: Copy + Eq + Hash, V: Compact + Clone, A: Allocator> Default for OpenAddressingMap<K, V, A> {
    fn default() -> Self {
        OpenAddressingMap::with_capacity(100)
    }
}

impl<K: Copy + Eq + Hash, V: Compact + Clone, A: Allocator> Default for CompactHashMap<K, V, A> {
    fn default() -> Self {
        CompactHashMap::new()
    }
}

impl<K: Copy + Eq + Hash, V: Compact + Clone, A: Allocator> ::std::iter::FromIterator<(K, V)>
    for OpenAddressingMap<K, V, A> {
    /// Construct a compact dictionary from an interator over key-value pairs
    fn from_iter<T: IntoIterator<Item = (K, V)>>(iter: T) -> Self {
        let mut map = Self::with_capacity(100);
        for (key, value) in iter {
            map.insert(key, value);
        }
        map
    }
}

impl<K: Copy + Eq + Hash, V: Compact + Clone, A: Allocator> ::std::iter::FromIterator<(K, V)>
    for CompactHashMap<K, V, A> {
    /// Construct a compact dictionary from an interator over key-value pairs
    fn from_iter<T: IntoIterator<Item = (K, V)>>(iter: T) -> Self {
        let mut map = Self::new();
        for (key, value) in iter {
            map.insert(key, value);
        }
        map
    }
}

struct OpenAddressingMapIter<K, V> {
    idx: usize,
    left: usize,
    _k: PhantomData<K>,
    _v: PhantomData<V>,
}

impl <K: Copy + Eq + Hash,V: Clone + Compact> Iterator for OpenAddressingMapIter<K, V> {
    type Item = Entry<K, V>;

    fn next(&mut self) -> Option<Entry<K, V>> {
        if self.left == 0 {
            return None;
        }

        loop {
            unsafe {
                let item = self.raw;
                self.idx += 1;
                if *item.hash() != EMPTY_BUCKET {
                    self.left -= 1;
                    return Some(item);
                }
            }
        }
    }
}


fn elem(n: u32) -> u32 {
    (n * n) as u32
}

#[test]
fn basic() {
    let n: u32 = 3000;
    let mut map: CompactHashMap<u32, u32> = CompactHashMap::new();
    assert!(map.is_empty() == true);
    for i in 0..n {
        map.insert(i, elem(i));
    }
    assert!(map.is_empty() == false);
    for i in 0..n {
        assert!(*map.get(i).unwrap() == i * i);
    }
    assert!(map.len() == n as usize);
    assert!(*map.get_mru(n - 1).unwrap() == elem(n - 1));
    assert!(*map.get_mfu(n - 100).unwrap() == elem(n - 100));
    assert!(map.contains_key(n - 300) == true);
    assert!(map.contains_key(n + 1) == false);
    assert!(map.remove(500) == Some(elem(500)));
    assert!(map.get_mru(500).is_none());
}

#[test]
fn iter() {
    let mut map: CompactHashMap<u32, u32> = CompactHashMap::new();
    assert!(map.is_empty() == true);
    for n in 0..100 {
        map.insert(n, n * n);
    }
    let mut sum = 0;
    let mut keys = map.keys();
    for n in 0..100 {
        assert!(keys.find(|i| **i == n).is_some());
    }
    let mut values = map.values();
    for n in 0..100 {
        assert!(values.find(|i| **i == elem(n)).is_some());
    }

}
#[test]
fn values_mut() {
    let mut map: CompactHashMap<u32, u32> = CompactHashMap::new();
    assert!(map.is_empty() == true);
    for n in 0..100 {
        map.insert(n, n * n);
    }
    {
        let mut values_mut = map.values_mut();
        for i in &mut values_mut {
            *i = *i + 1;
        }
    }
    for i in 0..100 {
        assert!(*map.get(i).unwrap() == i * i + 1);
    }
}

#[test]
fn pairs() {
    let mut map: CompactHashMap<u32, u32> = CompactHashMap::new();
    assert!(map.is_empty() == true);
    for n in 0..100 {
        map.insert(n, n * n);
    }
    for (key, value) in map.pairs() {
        assert!( elem(*key) == *value);
    }
}

#[test]
fn push_at() {
    let mut map: CompactHashMap<u32, CompactVec<u32>> = CompactHashMap::new();
    assert!(map.is_empty() == true);
    for n in 0..100 {
        map.push_at(n, elem(n));
        map.push_at(n, elem(n)+1);
    }
    
    for n in 0..100 {
        let mut iter = map.get_iter(n);
        assert!(iter.find(|i|{ **i == elem(n)}).is_some());
        assert!(iter.find(|i|{ **i == elem(n) + 1}).is_some());
    }
}

#[test]
fn remove_iter() {
    let mut map: CompactHashMap<u32, CompactVec<u32>> = CompactHashMap::new();
    assert!(map.is_empty() == true);
    for n in 0..100 {
        map.push_at(n, elem(n));
        map.push_at(n, elem(n)+1);
    }
    let mut iter = map.remove_iter(50);
    assert!(iter.find(|i|{ *i == elem(50)}).is_some());
    assert!(iter.find(|i|{ *i == elem(50) + 1}).is_some());
}