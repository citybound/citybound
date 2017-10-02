use super::allocators::{Allocator, DefaultHeap};
use super::pointer_to_maybe_compact::PointerToMaybeCompact;
use super::compact::Compact;
use std::marker::PhantomData;
use std::ptr;
use std::ops::{Deref, DerefMut};
use std::iter::FromIterator;

/// A dynamically-sized vector that can be stored in compact sequential storage and
/// automatically spills over into free heap storage using `Allocator`.
/// Tries to closely follow the API of `std::vec::Vec`, but is not complete.
pub struct CompactVec<T, A: Allocator = DefaultHeap> {
    /// Points to either compact or free storage
    ptr: PointerToMaybeCompact<T>,
    len: usize,
    /// Maximum capacity before needing to spill onto the heap
    cap: usize,
    _alloc: PhantomData<*const A>,
}

impl<T: Compact + Clone, A: Allocator> CompactVec<T, A> {
    /// Get the number of elements in the vector
    pub fn len(&self) -> usize {
        self.len
    }

    /// Is the vector empty?
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Create a new, empty vector
    pub fn new() -> CompactVec<T, A> {
        CompactVec {
            ptr: PointerToMaybeCompact::default(),
            len: 0,
            cap: 0,
            _alloc: PhantomData,
        }
    }

    /// Create a new, empty vector with a given capacity
    pub fn with_capacity(cap: usize) -> CompactVec<T, A> {
        let mut vec = CompactVec {
            ptr: PointerToMaybeCompact::default(),
            len: 0,
            cap: cap,
            _alloc: PhantomData,
        };

        vec.ptr.set_to_free(A::allocate::<T>(cap));
        vec
    }

    /// current capacity
    pub fn capacity(&self) -> usize {
        self.cap
    }

    /// Double the capacity of the vector by spilling onto the heap
    fn double_buf(&mut self) {
        let new_cap = if self.cap == 0 { 1 } else { self.cap * 2 };
        let new_ptr = A::allocate::<T>(new_cap);

        // items should be decompacted, else internal relative pointers get messed up!
        for (i, item) in self.iter().enumerate() {
            unsafe { ptr::write(new_ptr.offset(i as isize), Compact::decompact(item)) };
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

    /// Push an item onto the vector, spills onto the heap
    /// if the capacity in compact storage is insufficient
    pub fn push(&mut self, value: T) {
        if self.len == self.cap {
            self.double_buf();
        }

        unsafe {
            let end = self.as_mut_ptr().offset(self.len as isize);
            ptr::write(end, value);
            self.len += 1;
        }
    }

    /// push at position
    pub fn push_at(&mut self, _: usize, value: T) {
        if self.len == self.cap {
            self.double_buf();
        }

        unsafe {
            let end = self.as_mut_ptr().offset(self.len as isize);
            ptr::write(end, value);
            self.len += 1;
        }
    }

    /// Extend from a copyable slice
    pub fn extend_from_copy_slice(&mut self, other: &[T])
    where
        T: Copy,
    {
        while self.len + other.len() > self.cap {
            self.double_buf();
        }

        let old_len = self.len;
        self.len += other.len();
        self[old_len..].copy_from_slice(other);
    }

    /// Pop and return the last element, if the vector wasn't empty
    pub fn pop(&mut self) -> Option<T> {
        if self.len == 0 {
            None
        } else {
            unsafe {
                self.len -= 1;
                Some(Compact::decompact(self.get_unchecked(self.len())))
            }
        }
    }

    /// Insert a value at `index`, copying the elements after `index` upwards
    pub fn insert(&mut self, index: usize, value: T) {
        if self.len == self.cap {
            self.double_buf();
        }

        unsafe {
            // infallible
            {
                let ptr = self.as_mut_ptr().offset(index as isize);
                // elements should be decompacted, else internal relative pointers get messed up!
                for i in (0..self.len - index).rev() {
                    ptr::write(
                        ptr.offset((i + 1) as isize),
                        Compact::decompact(&self[index + i]),
                    );
                }
                ptr::write(ptr, value);
            }
            self.len += 1;
        }
    }

    /// Remove the element at `index`, copying the elements after `index` downwards
    pub fn remove(&mut self, index: usize) -> T {
        let len = self.len;
        assert!(index < len);
        unsafe {
            // infallible
            let ret;
            {
                // the place we are taking from.
                let ptr = self.as_mut_ptr().offset(index as isize);
                // copy it out, unsafely having a copy of the value on
                // the stack and in the vector at the same time.
                ret = Compact::decompact(ptr);

                // Shift everything down to fill in that spot.
                // elements should be decompacted, else internal relative pointers get messed up!
                for i in 0..len - index - 1 {
                    ptr::write(
                        ptr.offset(i as isize),
                        Compact::decompact(&self[index + i + 1]),
                    )
                }
            }
            self.len -= 1;
            ret
        }
    }

    /// Take a function which returns whether an element should be kept,
    /// and mutably removes all elements from the vector which are not kept
    pub fn retain<F: FnMut(&T) -> bool>(&mut self, mut keep: F) {
        let mut del = 0;
        let len = self.len();
        {
            let v = &mut **self;

            for i in 0..len {
                if !keep(&v[i]) {
                    del += 1;
                } else {
                    v.swap(i - del, i);
                }
            }
        }

        if del > 0 {
            self.truncate(len - del);
        }
    }

    /// Truncate the vector to the given length
    pub fn truncate(&mut self, desired_len: usize) {
        unsafe {
            while desired_len < self.len {
                self.len -= 1;
                let len = self.len;
                ptr::drop_in_place(self.get_unchecked_mut(len));
            }
        }
    }

    /// Clear the vector
    pub fn clear(&mut self) {
        self.truncate(0);
    }

    /// Drain (empty & iterate over) the vector
    pub fn drain(&mut self) -> IntoIter<T, A> {
        unsafe {
            let decompacted = Compact::decompact(self);
            ::std::ptr::write(self, CompactVec::new());
            decompacted.into_iter()
        }
    }

    /// debug printing
    pub fn ptr_to_string(&self) -> String {
        self.ptr.to_string()
    }
}

impl<T, A: Allocator> From<Vec<T>> for CompactVec<T, A> {
    /// Create a `CompactVec` from a normal `Vec`,
    /// directly using the backing storage as free heap storage
    fn from(mut vec: Vec<T>) -> Self {
        let p = vec.as_mut_ptr();
        let len = vec.len();
        let cap = vec.capacity();

        ::std::mem::forget(vec);

        CompactVec {
            ptr: PointerToMaybeCompact::new_free(p),
            len: len,
            cap: cap,
            _alloc: PhantomData,
        }
    }
}

impl<T, A: Allocator> Drop for CompactVec<T, A> {
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

impl<T, A: Allocator> Deref for CompactVec<T, A> {
    type Target = [T];

    fn deref(&self) -> &[T] {
        unsafe { ::std::slice::from_raw_parts(self.ptr.ptr(), self.len) }
    }
}

impl<T, A: Allocator> DerefMut for CompactVec<T, A> {
    fn deref_mut(&mut self) -> &mut [T] {
        unsafe { ::std::slice::from_raw_parts_mut(self.ptr.mut_ptr(), self.len) }
    }
}

pub struct IntoIter<T, A: Allocator> {
    ptr: PointerToMaybeCompact<T>,
    len: usize,
    cap: usize,
    index: usize,
    _alloc: PhantomData<*const A>,
}

impl<T, A: Allocator> Iterator for IntoIter<T, A> {
    type Item = T;

    fn next(&mut self) -> Option<T> {
        if self.index < self.len {
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
                self.len,
            ))
        };
        if !self.ptr.is_compact() {
            unsafe {
                A::deallocate(self.ptr.mut_ptr(), self.cap);
            }
        }
    }
}

impl<T, A: Allocator> IntoIterator for CompactVec<T, A> {
    type Item = T;
    type IntoIter = IntoIter<T, A>;

    fn into_iter(self) -> Self::IntoIter {
        let iter = IntoIter {
            ptr: unsafe { ptr::read(&self.ptr) },
            len: self.len,
            cap: self.cap,
            index: 0,
            _alloc: PhantomData,
        };
        ::std::mem::forget(self);
        iter
    }
}

impl<'a, T, A: Allocator> IntoIterator for &'a CompactVec<T, A> {
    type Item = &'a T;
    type IntoIter = ::std::slice::Iter<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl<'a, T, A: Allocator> IntoIterator for &'a mut CompactVec<T, A> {
    type Item = &'a mut T;
    type IntoIter = ::std::slice::IterMut<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter_mut()
    }
}

impl<T: Compact + Clone, A: Allocator> Compact for CompactVec<T, A> {
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
        (*dest).len = (*source).len;
        (*dest).cap = (*source).cap;
        (*dest).ptr.set_to_compact(new_dynamic_part as *mut T);

        let mut offset = (*source).cap * ::std::mem::size_of::<T>();

        for (i, item) in (*source).iter_mut().enumerate() {
            let size_of_this_item = item.dynamic_size_bytes();
            Compact::compact(
                item,
                &mut (*dest)[i],
                new_dynamic_part.offset(offset as isize),
            );
            offset += size_of_this_item;
        }

        // we want to free any allocated space,
        // but not semantically drop our contents (they just moved)
        if !(*source).ptr.is_compact() {
            A::deallocate((*source).ptr.mut_ptr(), (*source).cap);
        }
    }

    default unsafe fn decompact(source: *const Self) -> Self {
        if (*source).ptr.is_compact() {
            (*source)
                .iter()
                .map(|item| Compact::decompact(item))
                .collect()
        } else {
            CompactVec {
                ptr: ptr::read(&(*source).ptr as *const PointerToMaybeCompact<T>),
                len: (*source).len,
                cap: (*source).cap,
                _alloc: (*source)._alloc,
            }
            // caller has to make sure that self will not be dropped!
        }
    }
}

impl<T: Copy, A: Allocator> Compact for CompactVec<T, A> {
    fn is_still_compact(&self) -> bool {
        self.ptr.is_compact()
    }

    fn dynamic_size_bytes(&self) -> usize {
        self.cap * ::std::mem::size_of::<T>()
    }

    unsafe fn compact(source: *mut Self, dest: *mut Self, new_dynamic_part: *mut u8) {
        (*dest).len = (*source).len;
        (*dest).cap = (*source).cap;
        (*dest).ptr.set_to_compact(new_dynamic_part as *mut T);

        ptr::copy_nonoverlapping(
            (*source).ptr.ptr(),
            new_dynamic_part as *mut T,
            (*source).len,
        );

        // we want to free any allocated space,
        // but not semantically drop our contents (they just moved)
        if !(*source).ptr.is_compact() {
            A::deallocate((*source).ptr.mut_ptr(), (*source).cap);
        }
    }
}

impl<T: Clone, A: Allocator> Clone for CompactVec<T, A> {
    default fn clone(&self) -> CompactVec<T, A> {
        self.iter().cloned().collect::<Vec<_>>().into()
    }
}

impl<T: Copy, A: Allocator> Clone for CompactVec<T, A> {
    fn clone(&self) -> CompactVec<T, A> {
        let mut new_vec = Self::with_capacity(self.cap);
        unsafe {
            ptr::copy_nonoverlapping(self.ptr.ptr(), new_vec.ptr.mut_ptr(), self.len);
        }
        new_vec.len = self.len;
        new_vec
    }
}

impl<T: Compact + Clone, A: Allocator> FromIterator<T> for CompactVec<T, A> {
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        let into_iter = iter.into_iter();
        let mut vec = CompactVec::with_capacity(into_iter.size_hint().0);
        for item in into_iter {
            vec.push(item);
        }
        vec
    }
}

impl<T: Compact + Clone, A: Allocator> Extend<T> for CompactVec<T, A> {
    fn extend<I: IntoIterator<Item = T>>(&mut self, iter: I) {
        for item in iter {
            self.push(item);
        }
    }
}

impl<T: Compact, A: Allocator> Default for CompactVec<T, A> {
    fn default() -> CompactVec<T, A> {
        CompactVec::new()
    }
}

impl<T: Compact + ::std::fmt::Debug, A: Allocator> ::std::fmt::Debug for CompactVec<T, A> {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        (self.deref()).fmt(f)
    }
}

#[test]
fn basic_vector() {
    let mut list: CompactVec<u32> = CompactVec::new();

    list.push(1);
    list.push(2);
    list.push(3);

    assert_eq!(&[1, 2, 3], &*list);

    let bytes = list.total_size_bytes();
    let storage = DefaultHeap::allocate(bytes);

    unsafe {
        Compact::compact_behind(&mut list, storage as *mut CompactVec<u32>);
        ::std::mem::forget(list);
        assert_eq!(&[1, 2, 3], &**(storage as *mut CompactVec<u32>));
        println!("before decompact!");
        let decompacted = Compact::decompact(storage as *mut CompactVec<u32>);
        println!("after decompact!");
        assert_eq!(&[1, 2, 3], &*decompacted);
        DefaultHeap::deallocate(storage, bytes);
    }
}

#[test]
fn nested_vector() {
    type NestedType = CompactVec<CompactVec<u32>>;
    let mut list_of_lists: NestedType = CompactVec::new();

    list_of_lists.push(vec![1, 2, 3].into());
    list_of_lists.push(vec![4, 5, 6, 7, 8, 9].into());

    assert_eq!(&[1, 2, 3], &*list_of_lists[0]);
    assert_eq!(&[4, 5, 6, 7, 8, 9], &*list_of_lists[1]);

    let bytes = list_of_lists.total_size_bytes();
    let storage = DefaultHeap::allocate(bytes);

    unsafe {
        Compact::compact_behind(&mut list_of_lists, storage as *mut NestedType);
        ::std::mem::forget(list_of_lists);
        assert_eq!(&[1, 2, 3], &*(*(storage as *mut NestedType))[0]);
        assert_eq!(&[4, 5, 6, 7, 8, 9], &*(*(storage as *mut NestedType))[1]);
        println!("before decompact!");
        let decompacted = Compact::decompact(storage as *mut NestedType);
        println!("after decompact!");
        assert_eq!(&[1, 2, 3], &*decompacted[0]);
        assert_eq!(&[4, 5, 6, 7, 8, 9], &*decompacted[1]);
        DefaultHeap::deallocate(storage, bytes);
    }
}
