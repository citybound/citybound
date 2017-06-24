use super::allocators::{Allocator, DefaultHeap};
use super::compact::Compact;
use super::compact_vec::CompactVec;


#[derive(Clone)]
struct Entry<K: Copy, V: Compact + Clone> {
    key: K,
    value: V,
}

impl<K: Copy, V: Compact + Clone> Compact for Entry<K, V> {
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
        }
    }
}

pub struct CompactHashMap<K: Copy, V: Compact + Clone, A: Allocator = DefaultHeap> {
    keys: CompactVec<K, A>,
    values: CompactVec<V, A>,
    entries: CompactVec<Entry<K, V>, A>,
    size: usize,
}

impl<K: Eq + Copy, V: Compact + Clone, A: Allocator> CompactHashMap<K, V, A> {
    /// Create new, empty dictionary
    pub fn new() -> Self {
        CompactHashMap {
            keys: CompactVec::new(),
            values: CompactVec::new(),
            size: 0,
            entries: CompactVec::new(),
        }
    }

    /// Amount of entries in the dictionary
    pub fn len(&self) -> usize {
        self.keys.len()
    }

    /// Is the dictionary empty?
    pub fn is_empty(&self) -> bool {
        self.keys.is_empty()
    }

    /// Look up the value for key `query`, if it exists
    pub fn get(&self, query: K) -> Option<&V> {
        for i in 0..self.keys.len() {
            if self.keys[i] == query {
                return Some(&self.values[i]);
            }
        }
        None
    }

    /// Lookup up the value for key `query`, if it exists, but also swap the entry
    /// to the beginning of the key/value vectors, so a repeated lookup for that item will be faster
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

    /// Lookup up the value for key `query`, if it exists, but also swap the entry
    /// one index towards the beginning of the key/value vectors, so frequently repeated lookups
    /// for that item will be faster
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

    /// Does the dictionary contain a value for `query`?
    pub fn contains_key(&self, query: K) -> bool {
        self.keys.contains(&query)
    }

    /// Insert new value at key `query` and return the previous value at that key, if any existed
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

    /// Remove value at key `query` and return it, if it existed
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

    /// Iterator over all keys in the dictionary
    pub fn keys(&self) -> ::std::slice::Iter<K> {
        self.keys.iter()
    }

    /// Iterator over all values in the dictionary
    pub fn values(&self) -> ::std::slice::Iter<V> {
        self.values.iter()
    }

    /// Iterator over mutable references to all values in the dictionary
    pub fn values_mut(&mut self) -> ::std::slice::IterMut<V> {
        self.values.iter_mut()
    }

    /// Iterator over all key-value pairs in the dictionary
    #[allow(needless_lifetimes)]
    pub fn pairs<'a>(&'a self) -> impl Iterator<Item = (&'a K, &'a V)> + Clone + 'a {
        self.keys().zip(self.values())
    }
}

impl<K: Eq + Copy, I: Compact, A1: Allocator, A2: Allocator>
    CompactHashMap<K, CompactVec<I, A1>, A2> {
    /// Push a value onto the `CompactVec` at the key `query`
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

    /// Iterator over the `CompactVec` at the key `query`
    #[allow(needless_lifetimes)]
    pub fn get_iter<'a>(&'a self, query: K) -> impl Iterator<Item = &'a I> + 'a {
        self.get(query).into_iter().flat_map(|vec_in_option| {
            vec_in_option.iter()
        })
    }

    /// Remove the `CompactVec` at the key `query` and iterate over its elements (if it existed)
    #[allow(needless_lifetimes)]
    pub fn remove_iter<'a>(&'a mut self, query: K) -> impl Iterator<Item = I> + 'a {
        self.remove(query).into_iter().flat_map(|vec_in_option| {
            vec_in_option.into_iter()
        })
    }
}


impl<K: Copy, V: Compact + Clone, A: Allocator> Compact for CompactHashMap<K, V, A> {
    fn is_still_compact(&self) -> bool {
        self.keys.is_still_compact() && self.values.is_still_compact() &&
            self.entries.is_still_compact()
    }

    fn dynamic_size_bytes(&self) -> usize {
        self.keys.dynamic_size_bytes() + self.values.dynamic_size_bytes() +
            self.entries.dynamic_size_bytes()
    }

    unsafe fn compact_from(&mut self, source: &Self, new_dynamic_part: *mut u8) {
        self.keys.compact_from(&source.keys, new_dynamic_part);
        self.values.compact_from(
            &source.values,
            new_dynamic_part.offset(self.keys.dynamic_size_bytes() as isize),
        );
        self.entries.compact_from(
            &source.entries,
            new_dynamic_part.offset(
                (self.keys.dynamic_size_bytes() +
                     self.values.dynamic_size_bytes()) as isize,
            ),
        )
    }

    unsafe fn decompact(&self) -> CompactHashMap<K, V, A> {
        CompactHashMap {
            keys: self.keys.decompact(),
            values: self.values.decompact(),
            entries: self.entries.decompact(),
            size: self.size,
        }
    }
}

impl<K: Copy, V: Compact + Clone, A: Allocator> Clone for CompactHashMap<K, V, A> {
    fn clone(&self) -> Self {
        CompactHashMap {
            keys: self.keys.clone(),
            values: self.values.clone(),
            entries: self.entries.clone(),
            size: self.size,
        }
    }
}

impl<K: Copy + Eq, V: Compact + Clone, A: Allocator> Default for CompactHashMap<K, V, A> {
    fn default() -> Self {
        CompactHashMap::new()
    }
}

impl<K: Copy + Eq, V: Compact + Clone, A: Allocator> ::std::iter::FromIterator<(K, V)>
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