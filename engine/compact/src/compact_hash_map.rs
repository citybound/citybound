extern crate primal;

use super::allocators::{Allocator, DefaultHeap};
use super::compact::Compact;
use super::pointer_to_maybe_compact::PointerToMaybeCompact;
use super::compact_vec::CompactVec;
use std::iter::Iterator;
use std::collections::hash_map::DefaultHasher;
use std::hash::Hasher;
use std::hash::Hash;

use std::marker::PhantomData;
use std::ops::{Deref, DerefMut};
use std::fmt::Write;
use std;
use std::ptr;

#[derive(Clone)]
struct Entry<K, V> {
    hash: u32,
    tombstoned: bool,
    inner: Option<(K, V)>,
}

// to fix Copmact<T:Copy> clash
pub trait TrivialCompact {}

/// A dynamically-sized array that can be stored in compact sequential storage and
/// automatically spills over into free heap storage using `Allocator`.
#[derive(Default)]
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


struct QuadraticProbingIterator<'a, K: 'a, V: 'a, A: 'a + Allocator = DefaultHeap> {
    i: usize,
    len: usize,
    hash: u32,
    map: &'a OpenAddressingMap<K, V, A>,
}

struct QuadraticProbingMutIterator<'a, K: 'a, V: 'a, A: 'a + Allocator = DefaultHeap> {
    i: usize,
    len: usize,
    hash: u32,
    map: &'a mut OpenAddressingMap<K, V, A>,
}


/// A dynamically-sized open adressing quadratic probing hashmap
/// that can be stored in compact sequential storage and
/// automatically spills over into free heap storage using `Allocator`.
pub struct OpenAddressingMap<K, V, A: Allocator = DefaultHeap> {
    // TODO: this seems to represent something else than actual number of items
    //       figure out what and how it can be merged with len again
    internal_size: usize,
    len: usize,
    entries: CompactArray<Entry<K, V>, A>,
}

impl<K: Eq, V: Clone> Entry<K, V> {
    fn make_used(&mut self, hash: u32, key: K, value: V) {
        self.hash = hash;
        self.inner = Some((key, value));
    }

    fn replace_value(&mut self, new_val: V) -> Option<V> {
        debug_assert!(self.used());
        match self.inner.as_mut() {
            None => None,
            Some(kv) => {
                let old = kv.1.clone();
                kv.1 = new_val;
                Some(old)
            }
        }
    }

    fn remove(&mut self) -> Option<V> {
        let old_val = self.value_option().cloned();
        self.inner = None;
        self.tombstoned = true;
        old_val
    }

    fn used(&self) -> bool {
        self.tombstoned || self.inner.is_some()
    }

    fn alive(&self) -> bool {
        self.inner.is_some()
    }

    fn free(&self) -> bool {
        self.inner.is_none() && (!self.tombstoned)
    }

    fn key(&self) -> &K {
        &self.inner.as_ref().unwrap().0
    }

    fn value(&self) -> &V {
        self.inner.as_ref().map(|kv| &kv.1).unwrap()
    }

    fn value_option(&self) -> Option<&V> {
        self.inner.as_ref().map(|kv| &kv.1)
    }

    fn mut_value(&mut self) -> &mut V {
        self.inner.as_mut().map(|kv| &mut kv.1).unwrap()
    }

    fn mut_value_option(&mut self) -> Option<&mut V> {
        self.inner.as_mut().map(|kv| &mut kv.1)
    }

    fn is_this(&self, key: &K) -> bool {
        self.inner.as_ref().map_or(false, |kv| &kv.0 == key)
    }

    fn into_tuple(self) -> (K, V) {
        debug_assert!(self.alive());
        let kv = self.inner.unwrap();
        (kv.0, kv.1)
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
        Entry { hash: 0, tombstoned: false, inner: None }
    }
}

impl<K: Copy, V: Compact> Compact for Entry<K, V> {
    default fn is_still_compact(&self) -> bool {
        self.inner.as_ref().map_or(true, |kv_tuple| {
            kv_tuple.1.is_still_compact()
        })
    }

    default fn dynamic_size_bytes(&self) -> usize {
        self.inner.as_ref().map_or(0, |kv_tuple| {
            kv_tuple.1.dynamic_size_bytes()
        })
    }

    default unsafe fn compact(source: *mut Self, dest: *mut Self, new_dynamic_part: *mut u8) {
        (*dest).hash = (*source).hash;
        (*dest).tombstoned = (*source).tombstoned;
        ::std::ptr::copy_nonoverlapping(&mut (*source).inner, &mut (*dest).inner, 1);
        if (*dest).inner.is_some() {
            Compact::compact(
                &mut (*source).inner.as_mut().unwrap().1,
                &mut (*dest).inner.as_mut().unwrap().1,
                new_dynamic_part,
            )
        }
    }

    default unsafe fn decompact(source: *const Self) -> Entry<K, V> {
        if (*source).inner.is_none() {
            Entry {
                hash: (*source).hash,
                tombstoned: (*source).tombstoned,
                inner: None,
            }
        } else {
            let insides = (*source).inner.as_ref().unwrap();
            Entry {
                hash: (*source).hash,
                tombstoned: (*source).tombstoned,
                inner: Some((insides.0, (Compact::decompact(&insides.1)))),
            }
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

    unsafe fn compact(source: *mut Self, dest: *mut Self, _new_dynamic_part: *mut u8) {
        (*dest).hash = (*source).hash;
        (*dest).tombstoned = (*source).tombstoned;
        (*dest).inner = (*source).inner;
    }

    unsafe fn decompact(source: *const Self) -> Entry<K, V> {
        Entry {
            hash: (*source).hash,
            tombstoned: (*source).tombstoned,
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
    #[allow(new_without_default_derive)]
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

impl<'a, K, V, A: Allocator> QuadraticProbingIterator<'a, K, V, A> {
    fn for_map(
        map: &'a OpenAddressingMap<K, V, A>,
        hash: u32,
    ) -> QuadraticProbingIterator<K, V, A> {
        QuadraticProbingIterator {
            i: 0,
            len: map.entries.cap,
            hash: hash,
            map: map,
        }
    }
}

impl<'a, K, V, A: Allocator> QuadraticProbingMutIterator<'a, K, V, A> {
    fn for_map(
        map: &'a mut OpenAddressingMap<K, V, A>,
        hash: u32,
    ) -> QuadraticProbingMutIterator<K, V, A> {
        QuadraticProbingMutIterator {
            i: 0,
            len: map.entries.cap,
            hash: hash,
            map: map,
        }
    }
}

impl<'a, K, V, A: Allocator> Iterator for QuadraticProbingIterator<'a, K, V, A> {
    type Item = &'a Entry<K, V>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.i >= self.len {
            return None;
        }
        let index = (self.hash as usize + self.i * self.i) % self.len;
        self.i += 1;
        Some(&self.map.entries[index])
    }
}

impl<'a, K, V, A: Allocator> Iterator for QuadraticProbingMutIterator<'a, K, V, A> {
    type Item = &'a mut Entry<K, V>;
    fn next(&mut self) -> Option<&'a mut Entry<K, V>> {
        if self.i >= self.len {
            return None;
        }
        let index = (self.hash as usize + self.i * self.i) % self.len;
        self.i += 1;
        Some(unsafe { std::mem::transmute(&mut self.map.entries[index]) })
    }
}

impl<K: Copy + Eq + Hash, V: Compact, A: Allocator> OpenAddressingMap<K, V, A> {
    /// constructor
    pub fn new() -> Self {
        Self::with_capacity(4)
    }
    /// constructor
    pub fn with_capacity(l: usize) -> Self {
        OpenAddressingMap {
            entries: CompactArray::with_capacity(Self::find_prime_larger_than(l)),
            internal_size: 0,
            len: 0,
        }
    }

    /// Amount of entries in the dictionary
    pub fn len(&self) -> usize {
        self.len
    }

    /// Is the dictionary empty?
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Look up the value for key `query`, if it exists
    pub fn get(&self, query: K) -> Option<&V> {
        self.find_used(query).and_then(|e| e.value_option())
    }

    /// get mutable
    pub fn get_mut(&mut self, query: K) -> Option<&mut V> {
        self.find_used_mut(query).and_then(|e| e.mut_value_option())
    }

    /// Does the dictionary contain a value for `query`?
    pub fn contains_key(&self, query: K) -> bool {
        self.get(query).map_or(false, |_| true)
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
        self.entries.iter().filter(|e| e.alive()).map(|e| e.key())
    }

    /// Iterator over all values in the dictionary
    pub fn values<'a>(&'a self) -> impl Iterator<Item = &'a V> + 'a {
        self.entries.iter().filter(|e| e.alive()).map(|e| e.value())
    }

    /// Iterator over mutable references to all values in the dictionary
    pub fn values_mut<'a>(&'a mut self) -> impl Iterator<Item = &'a mut V> + 'a {
        self.entries.iter_mut().filter(|e| e.alive()).map(|e| {
            e.mut_value()
        })
    }

    /// Iterator over all key-value pairs in the dictionary
    pub fn pairs<'a>(&'a self) -> impl Iterator<Item = (&'a K, &'a V)> + 'a {
        self.entries.iter().filter(|e| e.alive()).map(|e| {
            (e.key(), e.value())
        })
    }

    fn hash(key: K) -> u32 {
        let mut hasher = DefaultHasher::new();
        key.hash(&mut hasher);
        hasher.finish() as u32
    }

    fn insert_inner_growing(&mut self, query: K, value: V) -> Option<V> {
        self.ensure_capacity();
        self.insert_inner(query, value)
    }

    fn insert_inner(&mut self, query: K, value: V) -> Option<V> {
        let res = self.insert_inner_inner(query, value);
        if res.is_none() {
            self.internal_size += 1;
            self.len += 1;
        }
        res
    }

    fn insert_inner_inner(&mut self, query: K, value: V) -> Option<V> {
        let hash = Self::hash(query);
        for entry in self.quadratic_iterator_mut(hash) {
            if entry.free() {
                entry.make_used(hash, query, value);
                return None;
            } else if entry.is_this(&query) {
                return entry.replace_value(value);
            }
        }
        panic!("should have place")
    }

    fn remove_inner(&mut self, query: K) -> Option<V> {
        // remove inner does not alter the size because of tombstones
        let old = self.remove_inner_inner(query);
        if old.is_some() {
            self.len -= 1;
        }
        old
    }

    fn remove_inner_inner(&mut self, query: K) -> Option<V> {
        let hash = Self::hash(query);
        for entry in self.quadratic_iterator_mut(hash) {
            if entry.is_this(&query) {
                return entry.remove();
            }
        }
        None
    }

    fn ensure_capacity(&mut self) {
        if self.internal_size > self.entries.capacity() / 2 {
            let old_entries = self.entries.clone();
            self.entries = CompactArray::with_capacity(
                Self::find_prime_larger_than(old_entries.capacity() * 2),
            );
            self.internal_size = 0;
            for entry in old_entries {
                if entry.alive() {
                    let tuple = entry.into_tuple();
                    self.insert(tuple.0, tuple.1);
                }
            }
        }
    }

    fn find_used(&self, query: K) -> Option<&Entry<K, V>> {
        for entry in self.quadratic_iterator(query) {
            if entry.is_this(&query) {
                return Some(entry);
            }
        }
        None
    }

    fn find_used_mut(&mut self, query: K) -> Option<&mut Entry<K, V>> {
        let h = Self::hash(query);
        for entry in self.quadratic_iterator_mut(h) {
            if entry.is_this(&query) {
                return Some(entry);
            }
        }
        None
    }

    fn quadratic_iterator(&self, query: K) -> QuadraticProbingIterator<K, V, A> {
        QuadraticProbingIterator::for_map(self, Self::hash(query))
    }

    fn quadratic_iterator_mut(&mut self, hash: u32) -> QuadraticProbingMutIterator<K, V, A> {
        QuadraticProbingMutIterator::for_map(self, hash)
    }

    fn find_prime_larger_than(n: usize) -> usize {
        PRIME_SIEVE.primes_from(n).find(|&i| i > n).unwrap()
    }

    fn display(&self) -> String {
        let mut res = String::new();
        writeln!(&mut res, "size: {:?}", self.internal_size).unwrap();
        let mut size_left: isize = self.internal_size as isize;
        for entry in self.entries.iter() {
            if entry.used() {
                size_left -= 1;
            }
            writeln!(&mut res, "  {:?} {:?}", entry.used(), entry.hash).unwrap();
        }
        writeln!(&mut res, "size_left : {:?}", size_left).unwrap();
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
        (*dest).internal_size = (*source).internal_size;
        (*dest).len = (*source).len;
        Compact::compact(
            &mut (*source).entries,
            &mut (*dest).entries,
            new_dynamic_part,
        );

    }

    unsafe fn decompact(source: *const Self) -> OpenAddressingMap<K, V, A> {
        OpenAddressingMap {
            entries: Compact::decompact(&(*source).entries),
            internal_size: (*source).internal_size,
            len: (*source).len,
        }
    }
}

impl<K: Copy, V: Clone, A: Allocator> Clone for OpenAddressingMap<K, V, A> {
    fn clone(&self) -> Self {
        OpenAddressingMap {
            entries: self.entries.clone(),
            internal_size: self.internal_size,
            len: self.len,
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
        if self.push_at_inner(query, item) {
            self.internal_size += 1;
            self.len += 1;
        }
    }

    /// return true if new value pushed
    fn push_at_inner(&mut self, query: K, item: I) -> bool {
        self.ensure_capacity();
        let hash = Self::hash(query);
        for entry in self.quadratic_iterator_mut(hash) {
            if entry.is_this(&query) {
                entry.mut_value().push(item);
                return false;
            } else if !entry.used() {
                let mut val = CompactVec::new();
                val.push(item);
                entry.make_used(hash, query, val);
                return true;
            }
        }
        println!("{:?}", self.display());
        panic!("should always have place");
    }

    /// Iterator over the `CompactVec` at the key `query`
    pub fn get_iter<'a>(&'a self, query: K) -> impl Iterator<Item = &'a I> + 'a {
        self.get(query).into_iter().flat_map(|vec_in_option| {
            vec_in_option.iter()
        })
    }

    /// Remove the `CompactVec` at the key `query` and iterate over its elements (if it existed)
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
    let arr2 = arr.clone();
    assert!(arr2.iter().find(|&i| *i == 0).is_some());
}

#[test]
fn very_basic1() {
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
    assert!(*map.get(n - 1).unwrap() == elem(n - 1));
    assert!(*map.get(n - 100).unwrap() == elem(n - 100));
    assert!(map.contains_key(n - 300) == true);
    assert!(map.contains_key(n + 1) == false);
    assert!(map.remove(500) == Some(elem(500)));
    assert!(map.get(500).is_none());
}

#[test]
fn iter() {
    let mut map: OpenAddressingMap<usize, usize> = OpenAddressingMap::with_capacity(200);
    let n = 10;
    assert!(map.is_empty() == true);
    for n in 0..n {
        map.insert(n, n * n);
    }
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
    for n in 0..1000 {
        map.push_at(n, elem(n));
        map.push_at(n, elem(n) + 1);
    }
    let target = 500;
    let mut iter = map.remove_iter(target);
    assert!(iter.find(|i| *i == elem(target)).is_some());
    assert!(iter.find(|i| *i == elem(target) + 1).is_some());
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

#[test]
fn insert_after_remove_works_same_hash() {

    // get 2 elems with the same hash
    let mut hashToUsize: HashMap<u32, usize> = HashMap::new();
    let mut bad_pair_opt = None;
    for i in 0..<usize>::max_value() {
        if i % 10000 == 0 {
            println!("i {}", i);
        }
        let hash = OpenAddressingMap::<usize, usize>::hash(i);
        if (hashToUsize.contains_key(&hash)) {
            let p: usize = *hashToUsize.get(&hash).unwrap();
            bad_pair_opt = Some((i, p));
            break;
        }
        hashToUsize.insert(hash, i);
    }


    type NestedType = OpenAddressingMap<usize, usize>;
    let mut map: NestedType = OpenAddressingMap::new();

    let bad_pair = bad_pair_opt.unwrap();
    println!("bad pair {:?}", bad_pair);
    map.insert(bad_pair.0, 1);
    println!("map {}", map.display());
    map.insert(bad_pair.1, 2);
    println!("map {}", map.display());
    map.remove(bad_pair.0);
    println!("map {}", map.display());
    map.insert(bad_pair.1, 3);
    println!("map {}", map.display());

    let mut n1 = 0;
    for (key, value) in map.pairs() {
        if *key == bad_pair.1 {
            n1 += 1;
        }
    }
    assert!(n1 == 1);

}

#[test]
fn compact_notcopy() {
    type NestedType = OpenAddressingMap<usize, CompactVec<usize>>;

    let mut map: NestedType = OpenAddressingMap::new();
    let assert_fun = |map: &NestedType, t: usize| {
        assert!(
            map.get(t)
                .unwrap()
                .into_iter()
                .find(|i| **i == elem(t))
                .is_some()
        )
    };

    for n in 0..1000 {
        map.push_at(n, elem(n));
        map.push_at(n, elem(n) + 1);
    }
    assert_fun(&map, 500);
    let bytes = map.total_size_bytes();
    let storage = DefaultHeap::allocate(bytes);
    unsafe {
        Compact::compact_behind(&mut map, storage as *mut NestedType);
        ::std::mem::forget(map);
        assert_fun(&(*(storage as *mut NestedType)), 449);
        let decompacted = Compact::decompact(storage as *mut NestedType);
        assert_fun(&decompacted, 449);
        DefaultHeap::deallocate(storage, bytes);
    }
}

#[test]
fn compact_copy() {
    type NestedType = OpenAddressingMap<usize, usize>;

    let mut map: NestedType = OpenAddressingMap::new();
    let assert_fun = |map: &NestedType, t: usize| assert!(map.get(t).is_some());

    for n in 0..1000 {
        map.insert(n, elem(n));
    }
    assert_fun(&map, 500);
    let bytes = map.total_size_bytes();
    let storage = DefaultHeap::allocate(bytes);
    unsafe {
        Compact::compact_behind(&mut map, storage as *mut NestedType);
        ::std::mem::forget(map);
        assert_fun(&(*(storage as *mut NestedType)), 449);
        let decompacted = Compact::decompact(storage as *mut NestedType);
        assert_fun(&decompacted, 449);
        DefaultHeap::deallocate(storage, bytes);
    }
}
