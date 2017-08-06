
extern crate primal;


use super::allocators::{Allocator, DefaultHeap};
use super::compact::Compact;
use super::pointer_to_maybe_compact::PointerToMaybeCompact;
use super::compact_vec::CompactVec;
use std::iter::{Iterator, Map};
use std::collections::hash_map::DefaultHasher;
use std::hash::Hasher;
use std::hash::Hash;

use std::marker::PhantomData;
use std::ops::{Deref, DerefMut};
use std::fmt::Write;
use std;
use std::ptr;
use std::iter::FromIterator;

#[derive(Clone)]
struct KeyValue<K, V> {
    key: K,
    value: V,
}

#[derive(Clone)]
struct CompactOption<T> {
    inner: Option<T>,
}

#[derive(Clone)]
struct Entry<K, V> {
    hash: u64,
    inner: CompactOption<KeyValue<K, V>>,
}



pub trait TrivialCompact {}

/// A dynamically-sized array that can be stored in compact sequential storage and
/// automatically spills over into free heap storage using `Allocator`.
struct CompactArray<T, A: Allocator = DefaultHeap> {
    /// Points to either compact or free storage
    ptr: PointerToMaybeCompact<T>,
    /// Maximum capacity before needing to spill onto the heap
    cap: usize,
    _alloc: PhantomData<*const A>,
}

pub struct IntoIter<T, A: Allocator> {
    ptr: PointerToMaybeCompact<T>,
    cap: usize,
    index: usize,
    _alloc: PhantomData<*const A>,
}

/// A dynamically-sized open adressing quadratic probing hashmap
/// that can be stored in compact sequential storage and
/// automatically spills over into free heap storage using `Allocator`.
pub struct OpenAddressingMap<K, V, A: Allocator = DefaultHeap> {
    size: usize,
    entries: CompactArray<Entry<K, V>, A>,
}


impl<K: Copy, V: Compact> Compact for KeyValue<K, V> {
    default fn is_still_compact(&self) -> bool {
        self.value.is_still_compact()
    }

    default fn dynamic_size_bytes(&self) -> usize {
        self.value.dynamic_size_bytes()
    }

    default unsafe fn compact(source: *mut Self, dest: *mut Self, new_dynamic_part: *mut u8) {
        (*dest).key = (*source).key;
        Compact::compact(&mut (*source).value, &mut (*dest).value, new_dynamic_part)
    }

    default unsafe fn decompact(source: *const Self) -> KeyValue<K, V> {
        KeyValue {
            key: (*source).key,
            value: Compact::decompact(&(*source).value),
        }
    }
}

impl<T> CompactOption<T> {
    fn is_some(&self) -> bool {
        self.inner.is_some()
    }

    fn is_none(&self) -> bool {
        self.inner.is_none()
    }

    fn none() -> CompactOption<T> {
        CompactOption { inner: None }
    }

    fn some(t: T) -> CompactOption<T> {
        CompactOption { inner: Some(t) }
    }

    fn as_mut(&mut self) -> Option<&mut T> {
        self.inner.as_mut()
    }

    fn map_or<U, F: FnOnce(T) -> U>(&self, default: U, f: F) -> U {
        self.inner.map_or(default, f)
    }
}

impl<T: Compact> Compact for CompactOption<T> {
    default fn is_still_compact(&self) -> bool {
        self.inner.map_or(true, |t| t.is_still_compact())
    }

    default fn dynamic_size_bytes(&self) -> usize {
        self.inner.map_or(0, |t| t.dynamic_size_bytes())
    }

    default unsafe fn compact(source: *mut Self, dest: *mut Self, new_dynamic_part: *mut u8) {
        if (*source).is_none() {
            *dest = *source;
        } else {
            Compact::compact(
                &mut (*source).inner.unwrap(),
                &mut (*dest).inner.unwrap(),
                new_dynamic_part,
            )
        }
    }

    default unsafe fn decompact(source: *const Self) -> CompactOption<T> {
        if (*source).is_none() {
            CompactOption::none()
        } else {
            CompactOption::some(Compact::decompact(&(*source).inner.unwrap()))
        }
    }
}

impl<K, V: Clone> Entry<K, V> {
    fn new_used(hash: u64, key: K, value: V) -> Entry<K, V> {
        Entry {
            hash: hash,
            inner: CompactOption::some(KeyValue { key: key, value: value }),
        }
    }
    fn replace_value(&mut self, new_val: V) -> Option<V> {
        match self.inner.as_mut() {
            None => None,
            Some(&mut kv) => {
                let old = kv.value.clone();
                kv.value = new_val;
                Some(old)
            }
        }
    }

    fn used(&self) -> bool {
        self.inner.is_some()
    }
    fn key<'a>(&self) -> &'a K {
        &self.inner.inner.unwrap().key
    }
    fn key_option(&self) -> Option<&K> {
        self.inner.inner.map(|kv| &kv.key)
    }
    fn value(&self) -> &V {
        self.inner.inner.map(|kv| &kv.value).unwrap()
    }
    fn value_option(&self) -> Option<&V> {
        self.inner.inner.map(|kv| &kv.value)
    }
    fn mut_value(&mut self) -> &mut V {
        self.inner.inner.map(|kv| &mut kv.value).unwrap()
    }
    fn mut_value_option(&mut self) -> Option<&mut V> {
        self.inner.inner.map(|kv| &mut kv.value)
    }
}

impl<K: Copy, V: Copy> TrivialCompact for Entry<K, V> {}

impl<K, V> std::fmt::Debug for Entry<K, V> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "Entry {:?}, {:?}", self.hash, self.inner.is_some())
    }
}

impl<K, V> Default for Entry<K, V> {
    fn default() -> Self {
        Entry { hash: 0, inner: CompactOption::none() }
    }
}

impl<K: Copy, V: Compact> Compact for Entry<K, V> {
    default fn is_still_compact(&self) -> bool {
        self.inner.is_still_compact()
    }

    default fn dynamic_size_bytes(&self) -> usize {
        self.inner.dynamic_size_bytes()
    }

    default unsafe fn compact(source: *mut Self, dest: *mut Self, new_dynamic_part: *mut u8) {
        (*dest).hash = (*source).hash;
        Compact::compact(&mut (*source).inner, &mut (*dest).inner, new_dynamic_part)
    }

    default unsafe fn decompact(source: *const Self) -> Entry<K, V> {
        Entry {
            hash: (*source).hash,
            inner: Compact::decompact(&(*source).inner),
        }
    }
}

impl<K: Copy, V: Copy> Compact for Entry<K, V> {
    fn is_still_compact(&self) -> bool {
        true
    }

    fn dynamic_size_bytes(&self) -> usize {
        0
    }

    unsafe fn compact(source: *mut Self, dest: *mut Self, new_dynamic_part: *mut u8) {
        (*dest).hash = (*source).hash;
        (*dest).inner = (*source).inner;
    }

    unsafe fn decompact(source: *const Self) -> Entry<K, V> {
        Entry {
            hash: (*source).hash,
            inner: (*source).inner,
        }
    }
}

impl<T: Default, A: Allocator> CompactArray<T, A> {
    /// Is the vector empty?
    pub fn is_empty(&self) -> bool {
        self.cap == 0
    }

    /// Create a new, empty vector
    pub fn new() -> CompactArray<T, A> {
        CompactArray {
            ptr: PointerToMaybeCompact::default(),
            cap: 0,
            _alloc: PhantomData,
        }
    }

    /// Create a new, empty vector with a given capacity
    pub fn with_capacity(cap: usize) -> CompactArray<T, A> {
        let mut vec = CompactArray {
            ptr: PointerToMaybeCompact::default(),
            cap: cap,
            _alloc: PhantomData,
        };

        vec.ptr.set_to_free(A::allocate::<T>(cap));

        for i in 0..cap {
            vec.ptr.init_with_default(i as isize);
        }

        vec
    }

    pub fn capacity(&self) -> usize {
        self.cap
    }
}

impl<T, A: Allocator> From<Vec<T>> for CompactArray<T, A> {
    /// Create a `CompactArray` from a normal `Vec`,
    /// directly using the backing storage as free heap storage
    fn from(mut vec: Vec<T>) -> Self {
        let p = vec.as_mut_ptr();
        let cap = vec.len();

        ::std::mem::forget(vec);

        CompactArray {
            ptr: PointerToMaybeCompact::new_free(p),
            cap: cap,
            _alloc: PhantomData,
        }
    }
}

impl<T, A: Allocator> Drop for CompactArray<T, A> {
    /// Drop elements and deallocate free heap storage, if any is allocated
    fn drop(&mut self) {
        unsafe { ptr::drop_in_place(&mut self[..]) };
        if !self.ptr.is_compact() {
            unsafe {
                A::deallocate(self.ptr.mut_ptr(), self.cap);
            }
        }
    }
}

impl<T, A: Allocator> Deref for CompactArray<T, A> {
    type Target = [T];

    fn deref(&self) -> &[T] {
        unsafe { ::std::slice::from_raw_parts(self.ptr.ptr(), self.cap) }
    }
}

impl<T, A: Allocator> DerefMut for CompactArray<T, A> {
    fn deref_mut(&mut self) -> &mut [T] {
        unsafe { ::std::slice::from_raw_parts_mut(self.ptr.mut_ptr(), self.cap) }
    }
}



impl<T, A: Allocator> Iterator for IntoIter<T, A> {
    type Item = T;

    fn next(&mut self) -> Option<T> {
        if self.index < self.cap {
            let item = unsafe { ptr::read(self.ptr.ptr().offset(self.index as isize)) };
            self.index += 1;
            Some(item)
        } else {
            None
        }
    }
}

impl<T, A: Allocator> Drop for IntoIter<T, A> {
    fn drop(&mut self) {
        // drop all remaining elements
        unsafe {
            ptr::drop_in_place(&mut ::std::slice::from_raw_parts(
                self.ptr.ptr().offset(self.index as isize),
                self.cap,
            ))
        };
        if !self.ptr.is_compact() {
            unsafe {
                A::deallocate(self.ptr.mut_ptr(), self.cap);
            }
        }
    }
}

impl<T, A: Allocator> IntoIterator for CompactArray<T, A> {
    type Item = T;
    type IntoIter = IntoIter<T, A>;

    fn into_iter(self) -> Self::IntoIter {
        let iter = IntoIter {
            ptr: unsafe { ptr::read(&self.ptr) },
            cap: self.cap,
            index: 0,
            _alloc: PhantomData,
        };
        ::std::mem::forget(self);
        iter
    }
}

impl<'a, T, A: Allocator> IntoIterator for &'a CompactArray<T, A> {
    type Item = &'a T;
    type IntoIter = ::std::slice::Iter<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl<'a, T, A: Allocator> IntoIterator for &'a mut CompactArray<T, A> {
    type Item = &'a mut T;
    type IntoIter = ::std::slice::IterMut<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter_mut()
    }
}

impl<T: Compact + Sized, A: Allocator> Compact for CompactArray<T, A> {
    default fn is_still_compact(&self) -> bool {
        self.ptr.is_compact() && self.iter().all(|elem| elem.is_still_compact())
    }

    default fn dynamic_size_bytes(&self) -> usize {
        self.cap * ::std::mem::size_of::<T>() +
            self.iter()
                .map(|elem| elem.dynamic_size_bytes())
                .sum::<usize>()
    }

    default unsafe fn compact(source: *mut Self, dest: *mut Self, new_dynamic_part: *mut u8) {
        (*dest).cap = (*source).cap;
        (*dest).ptr.set_to_compact(new_dynamic_part as *mut T);

        let mut offset = (*source).cap * ::std::mem::size_of::<T>();
        for i in 0..(*source).cap {
            Compact::compact(
                &mut (*source)[i],
                &mut (*dest)[i],
                new_dynamic_part.offset(offset as isize),
            );
            offset += (*source)[i].dynamic_size_bytes();
        }
    }

    default unsafe fn decompact(source: *const Self) -> Self {
        if (*source).ptr.is_compact() {
            (*source).clone()
        } else {
            CompactArray {
                ptr: ptr::read(&(*source).ptr as *const PointerToMaybeCompact<T>),
                cap: (*source).cap,
                _alloc: (*source)._alloc,
            }
            // caller has to make sure that self will not be dropped!
        }
    }
}

impl<T: TrivialCompact + Compact, A: Allocator> Compact for CompactArray<T, A> {
    fn is_still_compact(&self) -> bool {
        self.ptr.is_compact()
    }

    fn dynamic_size_bytes(&self) -> usize {
        self.cap * ::std::mem::size_of::<T>()
    }

    unsafe fn compact(source: *mut Self, dest: *mut Self, new_dynamic_part: *mut u8) {
        (*dest).cap = (*source).cap;
        (*dest).ptr.set_to_compact(new_dynamic_part as *mut T);
        ptr::copy_nonoverlapping((*source).ptr.ptr(), (*dest).ptr.mut_ptr(), (*source).cap);
    }
}

impl<T: Clone, A: Allocator> Clone for CompactArray<T, A> {
    default fn clone(&self) -> CompactArray<T, A> {
        self.iter().cloned().collect::<Vec<_>>().into()
    }
}

impl<T: Copy + Default, A: Allocator> Clone for CompactArray<T, A> {
    fn clone(&self) -> CompactArray<T, A> {
        let mut new_vec = Self::with_capacity(self.cap);
        unsafe {
            ptr::copy_nonoverlapping(self.ptr.ptr(), new_vec.ptr.mut_ptr(), self.cap);
        }
        new_vec
    }
}

impl<T: Compact + ::std::fmt::Debug, A: Allocator> ::std::fmt::Debug for CompactArray<T, A> {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        (self.deref()).fmt(f)
    }
}

lazy_static! {
    static ref PRIME_SIEVE: primal::Sieve = {
        primal::Sieve::new(1_000_000)
    };
}

impl<K: Copy + Eq + Hash, V: Compact, A: Allocator> OpenAddressingMap<K, V, A> {
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
        self.get_inner(query).and_then(|e| e.value_option())
    }

    pub fn get_mru(&self, query: K) -> Option<&V> {
        self.get_inner(query).and_then(|e| e.value_option())
    }

    pub fn get_mfu(&self, query: K) -> Option<&V> {
        self.get_inner(query).and_then(|e| e.value_option())
    }

    pub fn get_mut(&mut self, query: K) -> Option<&mut V> {
        self.get_inner_mut(query).and_then(|e| e.mut_value_option())
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
        self.entries.iter().filter(|e| e.used()).map(|e| e.key())
    }

    /// Iterator over all values in the dictionary
    pub fn values<'a>(&'a self) -> impl Iterator<Item = &'a V> + 'a {
        self.entries.iter().filter(|e| e.used()).map(|e| e.value())
    }

    /// Iterator over mutable references to all values in the dictionary
    pub fn values_mut<'a>(&'a mut self) -> impl Iterator<Item = &'a mut V> + 'a {
        self.entries.iter_mut().filter(|e| e.used()).map(|e| {
            e.mut_value()
        })
    }

    /// Iterator over all key-value pairs in the dictionary
    pub fn pairs<'a>(&'a self) -> impl Iterator<Item = (&'a K, &'a V)> + 'a {
        self.entries.iter().filter(|e| e.used()).map(|e| {
            (e.key(), e.value())
        })
    }

    fn hash(&self, key: K) -> u64 {
        let mut hasher = DefaultHasher::new();
        key.hash(&mut hasher);
        hasher.finish()
    }

    fn write(&mut self, i: usize, hash: u64, query: K, value: V) -> Option<V> {
        if !self.entries[i].used() {
            self.entries[i] = Entry::new_used(hash, query, value);
            self.size += 1;
            None
        } else {
            self.entries[i].replace_value(value)
        }
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
            if !self.entries[index].used() {
                return self.write(index, hash, query, value);
            } else if *self.entries[i].key() == query {
                return self.write(index, hash, query, value);
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

impl<K: Copy + Eq + Hash, V: Compact, A: Allocator> Compact for OpenAddressingMap<K, V, A> {
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

impl<K: Copy, V: Clone, A: Allocator> Clone for OpenAddressingMap<K, V, A> {
    fn clone(&self) -> Self {
        OpenAddressingMap {
            entries: self.entries.clone(),
            size: self.size,
        }
    }
}

impl<K: Copy + Eq + Hash, V: Compact, A: Allocator> Default for OpenAddressingMap<K, V, A> {
    fn default() -> Self {
        OpenAddressingMap::with_capacity(5)
    }
}

impl<K: Copy + Eq + Hash, V: Compact + Clone, A: Allocator> ::std::iter::FromIterator<(K, V)>
    for OpenAddressingMap<K, V, A> {
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

impl<K: Hash + Eq + Copy, I: Compact, A1: Allocator, A2: Allocator>
    OpenAddressingMap<K, CompactVec<I, A1>, A2> {
    /// Push a value onto the `CompactVec` at the key `query`
    pub fn push_at(&mut self, query: K, item: I) {
        self.ensure_capacity();
        let hash = self.hash(query);
        let index = self.find_pos(query);
        match index {
            Some(i) => {
                if self.entries[i].used {
                    self.entries[i].value.push(item);
                } else {
                    let mut val = CompactVec::new();
                    val.push(item);
                    self.write(i, hash, query, val);
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



fn elem(n: usize) -> usize {
    (n * n) as usize
}

#[test]
fn array_basic() {
    let mut arr: CompactArray<u32> = CompactArray::with_capacity(2);
    arr[0] = 5;
    assert!(arr[0] == 5);
    arr[1] = 4;
    assert!(arr[1] == 4);
    arr[0] = 6;
    arr[1] = 7;
    assert!(arr[0] == 6);
    assert!(arr[1] == 7);
}

#[test]
fn array_basic2() {
    let mut arr: CompactArray<u32> = CompactArray::with_capacity(3);
    arr[0] = 5;
    assert!(arr[0] == 5);
    arr[1] = 4;
    assert!(arr[1] == 4);
    arr[0] = 6;
    arr[1] = 7;
    assert!(arr[0] == 6);
    assert!(arr[1] == 7);
}

#[test]
fn array_find() {
    let mut arr: CompactArray<u32> = CompactArray::with_capacity(3);
    arr[0] = 5;
    arr[1] = 0;
    arr[2] = 6;
    assert!(arr.iter().find(|&i| *i == 0).is_some());
}

#[test]
fn array_clone() {
    let mut arr: CompactArray<u32> = CompactArray::with_capacity(3);
    arr[0] = 5;
    arr[1] = 0;
    arr[2] = 6;
    assert!(arr.iter().find(|&i| *i == 0).is_some());
    let mut arr2 = arr.clone();
    assert!(arr2.iter().find(|&i| *i == 0).is_some());
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
