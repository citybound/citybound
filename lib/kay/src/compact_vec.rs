use super::allocators::{Allocator, DefaultHeap};
use super::pointer_to_maybe_compact::PointerToMaybeCompact;
use super::compact::Compact;
use ::std::marker::PhantomData;
use ::std::ptr;
use ::std::ops::{Deref, DerefMut};
use ::std::iter::FromIterator;

/// A vector which can be compacted. Will usually contain the data in the dynamic part of the
/// compacted data structure, but if the size will overflow the dynamic part, it is moved to the
/// heap instead
pub struct CompactVec <T, A: Allocator = DefaultHeap> {
    /// Either a null pointer, an offset to the dynamic part of the data or a pointer to an array
    /// in the heap
    ptr: PointerToMaybeCompact<T>,
    /// Current length
    len: usize,
    /// Maximum length before needing to expand the heap
    cap: usize,
    _alloc: PhantomData<*const A>
}

impl<T: Compact + Clone, A: Allocator> CompactVec<T, A> {
    /// Get the length of the vector
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
            _alloc: PhantomData
        }
    }

    /// Initialize the vector with a set capacity
    pub fn with_capacity(cap: usize) -> CompactVec<T, A> {
        let mut vec = CompactVec {
            ptr: PointerToMaybeCompact::default(),
            len: 0,
            cap: cap,
            _alloc: PhantomData
        };

        vec.ptr.set_to_free(A::allocate::<T>(cap));
        vec
    }

    /// Double the cap by allocating from the heap
    fn double_buf(&mut self) {
        let new_cap = if self.cap == 0 {1} else {self.cap * 2};
        let new_ptr = A::allocate::<T>(new_cap);

        // items should be decompacted, else internal relative pointers get messed up!
        #[allow(needless_range_loop)]
        for i in 0..self.len() {
            unsafe {ptr::write(new_ptr.offset(i as isize), self[i].decompact())};
        }

        // items shouldn't be dropped here, they live on in the new backing store!
        if !self.ptr.is_compact() {
            unsafe {A::deallocate(self.ptr.mut_ptr(), self.cap);}
        }
        self.ptr.set_to_free(new_ptr);
        self.cap = new_cap;
    }

    /// Push new data to the end of the vector
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

    /// Remove and return the last element
    pub fn pop(&mut self) -> Option<T> {
        if self.len == 0 {
            None
        } else {
            unsafe {
                self.len -= 1;
                Some(ptr::read(self.get_unchecked(self.len())))
            }
        }
    }

    /// Insert an value at the index, pushing the values behind it back
    pub fn insert(&mut self, index: usize, value: T) {
        if self.len == self.cap {
            self.double_buf();
        }

        unsafe {
            // infallible
            {
                let ptr = self.as_mut_ptr().offset(index as isize);
                // items should be decompacted, else internal relative pointers get messed up!
                for i in (0..self.len - index).rev() {
                    ptr::write(ptr.offset((i + 1) as isize), self[index + i].decompact());
                }
                //ptr::copy(p, p.offset(1), self.len - index);
                ptr::write(ptr, value);
            }
            self.len += 1;
        }
    }

    /// Remove the item at the index, with the items at the back being copied forward
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
                ret = ptr::read(ptr);

                // Shift everything down to fill in that spot.
                // items should be decompacted, else internal relative pointers get messed up!
                #[allow(needless_range_loop)]
                for i in 0..len - index - 1 {
                    ptr::write(ptr.offset(i as isize), self[index + i + 1].decompact())
                }
                //ptr::copy(ptr.offset(1), ptr, len - index - 1);
            }
            self.len -= 1;
            ret
        }
    }

    /// Take a function which returns if the value should be kept, and creates a new list with
    /// those values
    pub fn retain<F: FnMut(&T) -> bool>(&mut self, mut keep: F) {
        let mut del = 0;
        let len = self.len();
        {
            let v = &mut **self;

            #[allow(needless_range_loop)]
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

    /// Truncate the vector to the length desired
    pub fn truncate(&mut self, desired_len: usize) {
        unsafe {
            while desired_len < self.len {
                self.len -= 1;
                let len = self.len;
                ptr::drop_in_place(self.get_unchecked_mut(len));
            }
        }
    }

    /// Empty the array
    pub fn clear(&mut self) {
        self.truncate(0);
    }
}

impl<T, A: Allocator> From<Vec<T>> for CompactVec<T, A> {
    /// Utility function to convert from `Vec` to `CompactVec`
    fn from(mut vec: Vec<T>) -> Self {
        let p = vec.as_mut_ptr();
        let len = vec.len();
        let cap = vec.capacity();

        ::std::mem::forget(vec);

        CompactVec{
            ptr: PointerToMaybeCompact::new_free(p),
            len: len,
            cap: cap,
            _alloc: PhantomData 
        }
    }
}

impl<T, A: Allocator> Drop for CompactVec<T, A> {
    /// Deallocate memory and drop elements when dropped
    fn drop(&mut self) {
        unsafe {ptr::drop_in_place(&mut self[..])};
        if !self.ptr.is_compact() {
            unsafe {A::deallocate(self.ptr.mut_ptr(), self.cap);}
        }
    }
}

impl<T, A: Allocator> Deref for CompactVec<T, A> {
    type Target = [T];

    fn deref(&self) -> &[T] {
        unsafe {
            ::std::slice::from_raw_parts(self.ptr.ptr(), self.len)
        }
    }
}

impl<T, A: Allocator> DerefMut for CompactVec<T, A> {
    fn deref_mut(&mut self) -> &mut [T] {
        unsafe {
            ::std::slice::from_raw_parts_mut(self.ptr.mut_ptr(), self.len)
        }
    }
}

pub struct IntoIter<T, A: Allocator> {
    ptr: PointerToMaybeCompact<T>,
    len: usize,
    cap: usize,
    index: usize,
    _alloc: PhantomData<*const A>
}

impl<T, A: Allocator> Iterator for IntoIter<T, A>{
    type Item=T;

    fn next(&mut self) -> Option<T> {
        if self.index < self.len {
            let item = unsafe{ptr::read(self.ptr.ptr().offset(self.index as isize))};
            self.index += 1;
            Some(item)
        } else {None}
    }
}

impl<T, A: Allocator> Drop for IntoIter<T, A> {
    fn drop(&mut self) {
        // drop all remaining elements
        unsafe {ptr::drop_in_place(&mut ::std::slice::from_raw_parts(
            self.ptr.ptr().offset(self.index as isize), self.len
        ))};
        if !self.ptr.is_compact() {
            unsafe {A::deallocate(self.ptr.mut_ptr(), self.cap);}
        }
    }
}

impl<T, A: Allocator> IntoIterator for CompactVec<T, A> {
    type Item = T;
    type IntoIter = IntoIter<T, A>;
    
    fn into_iter(self) -> Self::IntoIter {
        let iter = IntoIter{
            ptr: unsafe{ptr::read(&self.ptr)},
            len: self.len,
            cap: self.cap,
            index: 0,
            _alloc: PhantomData
        };
        ::std::mem::forget(self);
        iter
    }
}

impl<'a, T, A: Allocator> IntoIterator for &'a CompactVec<T, A> {
    type Item = &'a T;
    type IntoIter = ::std::slice::Iter<'a, T>;

    fn into_iter(self) -> Self::IntoIter {self.iter()}
}

impl<'a, T, A: Allocator> IntoIterator for &'a mut CompactVec<T, A> {
    type Item = &'a mut T;
    type IntoIter = ::std::slice::IterMut<'a, T>;

    fn into_iter(self) -> Self::IntoIter {self.iter_mut()}
}

impl<T: Compact + Clone, A: Allocator> Compact for CompactVec<T, A> {
    default fn is_still_compact(&self) -> bool {
        self.ptr.is_compact() && self.iter().all(|elem| elem.is_still_compact())
    }

    default fn dynamic_size_bytes(&self) -> usize {
        self.cap * ::std::mem::size_of::<T>() + self.iter().map(|elem| elem.dynamic_size_bytes()).sum::<usize>()
    }

    default unsafe fn compact_from(&mut self, source: &Self, new_dynamic_part: *mut u8) {
        self.cap = source.cap;
        self.len = source.len;
        self.ptr.set_to_compact(new_dynamic_part as *mut T);
        
        let mut offset = self.cap * ::std::mem::size_of::<T>();
        for i in 0..self.len {
            self[i].compact_from(&source[i], new_dynamic_part.offset(offset as isize));
            offset += self[i].dynamic_size_bytes();
        }
    }

    default unsafe fn decompact(&self) -> Self {
        if self.ptr.is_compact() {
            self.clone()
        } else {
            CompactVec{
                ptr: ptr::read(&self.ptr as *const PointerToMaybeCompact<T>),
                len: self.len,
                cap: self.cap,
                _alloc: self._alloc
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

    unsafe fn compact_from(&mut self, source: &Self, new_dynamic_part: *mut u8) {
        self.cap = source.cap;
        self.len = source.len;
        self.ptr.set_to_compact(new_dynamic_part as *mut T);
        ptr::copy_nonoverlapping(source.ptr.ptr(), self.ptr.mut_ptr(), self.len);
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
    fn from_iter<I: IntoIterator<Item=T>>(iter: I) -> Self {
        let into_iter = iter.into_iter();
        let mut vec = CompactVec::with_capacity(into_iter.size_hint().0);
        for item in into_iter {
            vec.push(item);
        }
        vec
    }
}

impl<T: Compact + Clone, A: Allocator> Extend<T> for CompactVec<T, A> {
    fn extend<I: IntoIterator<Item=T>>(&mut self, iter: I) {
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