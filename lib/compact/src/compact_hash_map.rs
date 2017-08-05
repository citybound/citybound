use super::allocators::{Allocator, DefaultHeap};
use super::compact::Compact;
use super::compact_vec::CompactVec;
use super::compact_array::{CompactArray, IntoIter as ArrayIntoIter};
use std::iter::{Iterator, Map};
use std::collections::hash_map::DefaultHasher;
use std::hash::Hasher;
use std::hash::Hash;
use std::marker::PhantomData;
use std::fmt::Write;
use std;

extern crate primal;

#[derive(Clone)]
struct Entry<K, V> {
    key: K,
    hash: u64,
    value: V,
    used: bool,
}

impl<K, V> std::fmt::Debug for Entry<K, V> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "Entry {:?}, {:?}", self.hash, self.used)
    }
}

impl<K, V> Entry<K, V> {
    fn is_still_compact_const(&self) -> bool {
        true
    }
}

impl<K, V: Compact> Entry<K, V> {
    fn is_still_compact_inner_default(&self) -> bool {
        !self.used || self.value.is_still_compact()
    }
}

impl<K: Copy, V: Compact> Compact for Entry<K, V> {
    default fn is_still_compact(&self) -> bool {
        Self::is_still_compact_inner_default(&self)
    }

    default fn dynamic_size_bytes(&self) -> usize {
        self.value.dynamic_size_bytes()
    }

    default unsafe fn compact(source: *mut Self, dest: *mut Self, new_dynamic_part: *mut u8) {
        (*dest).key = (*source).key;
        (*dest).hash = (*source).hash;
        (*dest).used = (*source).used;
        Compact::compact(&mut (*source).value, &mut (*dest).value, new_dynamic_part);
    }

    default unsafe fn decompact(source: *const Self) -> Entry<K, V> {
        Entry {
            key: (*source).key.clone(),
            value: Compact::decompact(&(*source).value),
            hash: (*source).hash,
            used: (*source).used,
        }
    }
}

impl<K: Copy, V: Copy> Compact for Entry<K, V> {
    fn is_still_compact(&self) -> bool {
        Self::is_still_compact_const()
    }

    fn dynamic_size_bytes(&self) -> usize {
        0
    }

    unsafe fn compact(source: *mut Self, dest: *mut Self, new_dynamic_part: *mut u8) {
        (*dest).key = (*source).key;
        (*dest).hash = (*source).hash;
        (*dest).used = (*source).used;
        (*dest).value = (*source).value;
    }

    unsafe fn decompact(source: *const Self) -> Entry<K, V> {
        Entry {
            key: (*source).key,
            value: (*source).value,
            hash: (*source).hash,
            used: (*source).used,
        }
    }
}

impl<K: Default, V: Default> Default for Entry<K, V> {
    fn default() -> Self {
        Entry {
            key: K::default(),
            value: V::default(),
            hash: 0,
            used: false,
        }
    }
}

pub struct OpenAddressingMap<K, V, A: Allocator = DefaultHeap> {
    size: usize,
    entries: CompactArray<Entry<K, V>, A>,
}

lazy_static! {
    static ref PRIME_SIEVE: primal::Sieve = {
        primal::Sieve::new(1_000_000)
    };
}

impl<K: Copy + Eq + Hash + Default, V: Compact + Default, A: Allocator> OpenAddressingMap<K, V, A> {
    pub fn new() -> Self {
        Self::with_capacity(4)
    }
    pub fn with_capacity(l: usize) -> Self {
        let mut map = OpenAddressingMap {
            entries: CompactArray::with_capacity(Self::find_prime_larger_than(l)),
            size: 0,
        };
        map
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
        self.get_inner(query).map(|e| &e.value)
    }

    pub fn get_mru(&self, query: K) -> Option<&V> {
        self.get_inner(query).map(|e| &e.value)
    }

    pub fn get_mfu(&self, query: K) -> Option<&V> {
        self.get_inner(query).map(|e| &e.value)
    }

    pub fn get_mut(&mut self, query: K) -> Option<&mut V> {
        self.get_inner_mut(query).map(|e| &mut e.value)
    }

    /// Does the dictionary contain a value for `query`?
    pub fn contains_key(&self, query: K) -> bool {
        self.get(query).map_or(false, |i| true)
    }

    /// Insert new value at key `query` and return the previous value at that key, if any existed
    pub fn insert(&mut self, query: K, value: V) -> Option<V> {
        self.insert_inner_growing(query, value)
    }

    /// Remove value at key `query` and return it, if it existed
    pub fn remove(&mut self, query: K) -> Option<V> {
        self.remove_inner(query)
    }

    /// Iterator over all keys in the dictionary
    pub fn keys<'a>(&'a self) -> impl Iterator<Item = &'a K> + 'a {
        self.entries.iter().filter(|e| e.used).map(|e| (&e.key))
    }

    /// Iterator over all values in the dictionary
    pub fn values<'a>(&'a self) -> impl Iterator<Item = &'a V> + 'a {
        self.entries.iter().filter(|e| e.used).map(|e| (&e.value))
    }

    /// Iterator over mutable references to all values in the dictionary
    pub fn values_mut<'a>(&'a mut self) -> impl Iterator<Item = &'a mut V> + 'a {
        self.entries.iter_mut().filter(|e| e.used).map(
            |e| &mut e.value,
        )
    }

    /// Iterator over all key-value pairs in the dictionary
    pub fn pairs<'a>(&'a self) -> impl Iterator<Item = (&'a K, &'a V)> + 'a {
        self.entries.iter().filter(|e| e.used).map(
            |e| (&e.key, &e.value),
        )
    }

    fn hash(&self, key: K) -> u64 {
        let mut hasher = DefaultHasher::new();
        key.hash(&mut hasher);
        hasher.finish()
    }

    fn get_inner(&self, query: K) -> Option<&Entry<K, V>> {
        self.find_pos_used(query).map(move |i| &self.entries[i])
    }

    fn get_inner_mut(&mut self, query: K) -> Option<&mut Entry<K, V>> {
        self.find_pos(query).map(move |i| &mut self.entries[i])
    }

    fn insert_inner_growing(&mut self, query: K, value: V) -> Option<V> {
        self.ensure_capacity();
        self.insert_inner(query, value)
    }

    fn insert_inner(&mut self, query: K, value: V) -> Option<V> {
        let len = self.entries.capacity();
        let hash = self.hash(query);
        let h = hash as usize;
        for i in 0..len {
            let index = (h + i * i) % len;
            let entry = &mut self.entries[index];
            if !entry.used {
                entry.key = query;
                entry.value = value;
                entry.used = true;
                entry.hash = hash;
                self.size += 1;
                return None;
            } else if entry.key == query {
                let old_val: V = entry.value.clone();
                entry.value = value;
                entry.hash = hash;
                return Some(old_val);
            } else {
            }
        }
        panic!("should have place")
    }



    fn remove_inner(&mut self, query: K) -> Option<V> {
        let len = self.entries.capacity();
        let h = self.hash(query) as usize;
        for i in 0..len {
            let index = (h + i * i) % len;
            if self.entries[index].used && (self.entries[index].key == query) {
                self.entries[index].used = false;
                self.size -= 1;
                return Some(self.entries[index].value.clone());
            }
        }
        None
    }

    fn ensure_capacity(&mut self) {
        if self.size > self.entries.capacity() / 2 {
            let old_entries = self.entries.clone();
            self.entries = CompactArray::with_capacity(
                Self::find_prime_larger_than(old_entries.capacity() * 2),
            );
            self.size = 0;
            for entry in old_entries {
                if entry.used {
                    self.insert(entry.key, entry.value);
                }
            }
        }
    }

    fn find_pos_used(&self, query: K) -> Option<usize> {
        self.find_pos(query).and_then(
            |i| match self.entries[i].used {
                true => Some(i),
                false => None,
            },
        )
    }

    fn find_pos(&self, query: K) -> Option<usize> {
        let len = self.entries.capacity();
        let h = self.hash(query) as usize;
        for i in 0..len {
            let index = (h + i * i) % len;
            let entry = &self.entries[index];
            if entry.used && (entry.key == query) {
                return Some(index);
            } else if !entry.used {
                return Some(index);
            }
        }
        None
    }

    fn find_prime_larger_than(n: usize) -> usize {
        PRIME_SIEVE.primes_from(n).find(|&i| i > n).unwrap()
    }

    pub fn display(&self) -> String {
        let mut res = String::new();
        writeln!(&mut res, "size: {:?}", self.size);
        let mut size_left: isize = self.size as isize;
        for entry in self.entries.iter() {
            if entry.used {
                size_left -= 1;
            }
            writeln!(&mut res, "  {:?} {:?}", entry.used, entry.hash).unwrap();
        }
        writeln!(&mut res, "size_left : {:?}", size_left);
        res
    }
}

impl<K: Copy + Eq + Hash + Default, V: Compact + Default, A: Allocator> Compact
    for OpenAddressingMap<K, V, A> {
    default fn is_still_compact(&self) -> bool {
        self.entries.is_still_compact()
    }

    default fn dynamic_size_bytes(&self) -> usize {
        self.entries.dynamic_size_bytes()
    }

    default unsafe fn compact(source: *mut Self, dest: *mut Self, new_dynamic_part: *mut u8) {
        (*dest).size = (*source).size;
        Compact::compact(
            &mut (*source).entries,
            &mut (*dest).entries,
            new_dynamic_part,
        );

    }

    unsafe fn decompact(source: *const Self) -> OpenAddressingMap<K, V, A> {
        OpenAddressingMap {
            entries: Compact::decompact(&(*source).entries),
            size: (*source).size,
        }
    }
}

impl<K: Copy + Default, V: Clone, A: Allocator> Clone for OpenAddressingMap<K, V, A> {
    fn clone(&self) -> Self {
        OpenAddressingMap {
            entries: self.entries.clone(),
            size: self.size,
        }
    }
}

impl<K: Copy + Eq + Hash + Default, V: Compact + Default, A: Allocator> Default
    for OpenAddressingMap<K, V, A> {
    fn default() -> Self {
        OpenAddressingMap::with_capacity(4)
    }
}

impl<
    K: Copy + Eq + Hash + Default,
    V: Compact + Clone + Default,
    A: Allocator,
> ::std::iter::FromIterator<(K, V)> for OpenAddressingMap<K, V, A> {
    /// Construct a compact dictionary from an interator over key-value pairs
    fn from_iter<T: IntoIterator<Item = (K, V)>>(iter_to_be: T) -> Self {
        let iter = iter_to_be.into_iter();
        let mut map = Self::with_capacity(iter.size_hint().0);
        for (key, value) in iter {
            map.insert(key, value);
        }
        map
    }
}

impl<K: Hash + Eq + Copy + Default, I: Compact, A1: Allocator, A2: Allocator>
    OpenAddressingMap<K, CompactVec<I, A1>, A2> {
    /// Push a value onto the `CompactVec` at the key `query`
    pub fn push_at(&mut self, query: K, item: I) {
        self.ensure_capacity();
        let index = self.find_pos(query);
        match index {
            Some(i) => {
                if self.entries[i].used {
                    self.entries[i].value.push(item);
                } else {
                    self.entries[i].used = true;
                    self.entries[i].value = CompactVec::new();
                    self.entries[i].value.push(item);
                    self.entries[i].hash = self.hash(query);
                    self.entries[i].key = query;
                    self.size += 1;
                }
            }
            None => {
                println!("{:?}", self.display());
                panic!("should always have place");
            }
        }
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

impl<T: Hash> Hash for CompactVec<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        for elem in self {
            elem.hash(state);
        }
    }
}

#[test]
fn elem(n: usize) -> usize {
    (n * n) as usize
}

#[test]
fn very_basic() {
    let mut map: OpenAddressingMap<u32, u32> = OpenAddressingMap::with_capacity(2);
    map.insert(0, 54);
    assert!(*map.get(0).unwrap() == 54);
    map.insert(1, 48);
    assert!(*map.get(1).unwrap() == 48);
}

#[test]
fn very_basic2() {
    let mut map: OpenAddressingMap<u32, u32> = OpenAddressingMap::with_capacity(3);
    map.insert(0, 54);
    map.insert(1, 48);
    assert!(*map.get(0).unwrap() == 54);
    assert!(*map.get(1).unwrap() == 48);
}


#[test]
fn basic() {
    let n: usize = 10000;
    let mut map: OpenAddressingMap<usize, usize> = OpenAddressingMap::with_capacity(n);
    assert!(map.is_empty() == true);
    for i in 0..n {
        let e = elem(i);
        map.insert(i, e);
    }
    assert!(map.is_empty() == false);
    for i in 0..n {
        let test = map.get(i).unwrap();
        let exp = elem(i);
        assert!(*test == exp, " failed exp {:?}  was {:?}", exp, test);
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
    let mut map: OpenAddressingMap<usize, usize> = OpenAddressingMap::with_capacity(200);
    let n = 10;
    assert!(map.is_empty() == true);
    for n in 0..n {
        map.insert(n, n * n);
    }
    let mut sum = 0;
    for k in map.keys() {
        println!(" k {:?}", k);
    }
    for n in 0..n {
        let mut keys = map.keys();
        assert!(
            keys.find(|&i| {
                println!("find {:?} {:?}", i, n);
                *i == n
            }).is_some(),
            "fail n {:?} ",
            n
        );
    }
    for n in 0..n {
        let mut values = map.values();
        assert!(values.find(|i| **i == elem(n)).is_some());
    }

}

#[test]
fn values_mut() {
    let mut map: OpenAddressingMap<usize, usize> = OpenAddressingMap::new();
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
    let mut map: OpenAddressingMap<usize, usize> = OpenAddressingMap::new();
    assert!(map.is_empty() == true);
    for n in 0..100 {
        map.insert(n, n * n);
    }
    for (key, value) in map.pairs() {
        assert!(elem(*key) == *value);
    }
}

#[test]
fn push_at() {
    let mut map: OpenAddressingMap<usize, CompactVec<usize>> = OpenAddressingMap::new();
    assert!(map.is_empty() == true);
    for n in 0..10000 {
        map.push_at(n, elem(n));
        map.push_at(n, elem(n) + 1);
    }

    for n in 0..10000 {
        println!("n {:?}", n);
        let mut iter = map.get_iter(n);
        assert!(iter.find(|&i| *i == elem(n)).is_some());
        let mut iter2 = map.get_iter(n);
        assert!(iter2.find(|&i| *i == elem(n) + 1).is_some());
    }
}

#[test]
fn remove_iter() {
    let mut map: OpenAddressingMap<usize, CompactVec<usize>> = OpenAddressingMap::new();
    assert!(map.is_empty() == true);
    for n in 0..100 {
        map.push_at(n, elem(n));
        map.push_at(n, elem(n) + 1);
    }
    let mut iter = map.remove_iter(50);
    assert!(iter.find(|i| *i == elem(50)).is_some());
    assert!(iter.find(|i| *i == elem(50) + 1).is_some());
}

#[test]
fn ensure_capacity_works() {
    let mut map: OpenAddressingMap<usize, CompactVec<usize>> = OpenAddressingMap::new();
    assert!(map.is_empty() == true);
    for n in 0..100 {
        map.push_at(n, elem(n));
        map.push_at(n, elem(n) + 1);
    }
    assert!(map.is_empty() == false);
}
