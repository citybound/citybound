use super::allocators::{Allocator, DefaultHeap};
use super::pointer_to_maybe_compact::PointerToMaybeCompact;
use super::compact::Compact;
use std::marker::PhantomData;
use std::ptr;
use std::ops::{Deref, DerefMut};
use std::iter::FromIterator;

/// A dynamically-sized array that can be stored in compact sequential storage and
/// automatically spills over into free heap storage using `Allocator`.
pub struct CompactArray<T, A: Allocator = DefaultHeap> {
    /// Points to either compact or free storage
    ptr: PointerToMaybeCompact<T>,
    /// Maximum capacity before needing to spill onto the heap
    cap: usize,
    _alloc: PhantomData<*const A>,
}

impl<T: Compact + Clone, A: Allocator> CompactArray<T, A> {
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
        vec
    }

    pub fn capacity(&self) -> usize {
        self.cap
    }

    /// Double the capacity of the array by spilling onto the heap
    #[allow(needless_range_loop)]
    pub fn double_buf(&mut self) {
        let new_cap = if self.cap == 0 { 1 } else { self.cap * 2 };
        let new_ptr = A::allocate::<T>(new_cap);

        // items should be decompacted, else internal relative pointers get messed up!
        for i in 0..self.cap {
            unsafe { ptr::write(new_ptr.offset(i as isize), self[i].decompact()) };
        }

        // items shouldn't be dropped here, they live on in the new backing store!
        if !self.ptr.is_compact() {
            unsafe {
                A::deallocate(self.ptr.mut_ptr(), self.cap);
            }
        }
        self.ptr.set_to_free(new_ptr);
        self.cap = new_cap;
    }

    pub fn push_at(&mut self, i: usize, value: T) {
        while i >= self.cap {
            self.double_buf();
        }

        unsafe {
            let end = self.as_mut_ptr().offset(i as isize);
            ptr::write(end, value);
        }
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

pub struct IntoIter<T, A: Allocator> {
    ptr: PointerToMaybeCompact<T>,
    cap: usize,
    index: usize,
    _alloc: PhantomData<*const A>,
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

impl<T: Compact + Clone, A: Allocator> Compact for CompactArray<T, A> {
    default fn is_still_compact(&self) -> bool {
        self.ptr.is_compact() && self.iter().all(|elem| elem.is_still_compact())
    }

    default fn dynamic_size_bytes(&self) -> usize {
        self.cap * ::std::mem::size_of::<T>() +
            self.iter()
                .map(|elem| elem.dynamic_size_bytes())
                .sum::<usize>()
    }

    default unsafe fn compact_from(&mut self, source: &Self, new_dynamic_part: *mut u8) {
        self.cap = source.cap;
        self.ptr.set_to_compact(new_dynamic_part as *mut T);

        let mut offset = self.cap * ::std::mem::size_of::<T>();
        for i in 0..self.cap {
            self[i].compact_from(&source[i], new_dynamic_part.offset(offset as isize));
            offset += self[i].dynamic_size_bytes();
        }
    }

    default unsafe fn decompact(&self) -> Self {
        if self.ptr.is_compact() {
            self.clone()
        } else {
            CompactArray {
                ptr: ptr::read(&self.ptr as *const PointerToMaybeCompact<T>),
                cap: self.cap,
                _alloc: self._alloc,
            }
            // caller has to make sure that self will not be dropped!
        }
    }
}

impl<T: Copy, A: Allocator> Compact for CompactArray<T, A> {
    fn is_still_compact(&self) -> bool {
        self.ptr.is_compact()
    }

    fn dynamic_size_bytes(&self) -> usize {
        self.cap * ::std::mem::size_of::<T>()
    }

    unsafe fn compact_from(&mut self, source: &Self, new_dynamic_part: *mut u8) {
        self.cap = source.cap;
        self.ptr.set_to_compact(new_dynamic_part as *mut T);
        ptr::copy_nonoverlapping(source.ptr.ptr(), self.ptr.mut_ptr(), self.cap);
    }
}

impl<T: Clone, A: Allocator> Clone for CompactArray<T, A> {
    default fn clone(&self) -> CompactArray<T, A> {
        self.iter().cloned().collect::<Vec<_>>().into()
    }
}

impl<T: Copy, A: Allocator> Clone for CompactArray<T, A> {
    fn clone(&self) -> CompactArray<T, A> {
        let mut new_vec = Self::with_capacity(self.cap);
        unsafe {
            ptr::copy_nonoverlapping(self.ptr.ptr(), new_vec.ptr.mut_ptr(), self.cap);
        }
        new_vec
    }
}

impl<T: Compact + Clone, A: Allocator> FromIterator<T> for CompactArray<T, A> {
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        let into_iter = iter.into_iter();
        let mut vec = CompactArray::with_capacity(into_iter.size_hint().0);
        let mut i = 0;
        for item in into_iter {
            vec.push_at(i, item);
            i = i + 1;
        }
        vec
    }
}

impl<T: Compact + Clone, A: Allocator> Extend<T> for CompactArray<T, A> {
    fn extend<I: IntoIterator<Item = T>>(&mut self, iter: I) {
        let mut i = 0;
        for item in iter {
            self.push_at(0, item);
            i = i + 1;
        }
    }
}

impl<T: Compact, A: Allocator> Default for CompactArray<T, A> {
    fn default() -> CompactArray<T, A> {
        CompactArray::new()
    }
}

impl<T: Compact + ::std::fmt::Debug, A: Allocator> ::std::fmt::Debug for CompactArray<T, A> {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        (self.deref()).fmt(f)
    }
}

#[test]
fn basic() {
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
fn basic2() {
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