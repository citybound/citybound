#![allow(dead_code)]
use std::mem;
use std::mem::transmute;
use std::ptr;
use std::marker::PhantomData;
use std::ops::{Deref, DerefMut};

enum DropBehaviour {
    ShouldDrop,
    NoDrop
}

struct MaybeNonDropPointer <T> {
    raw_ptr: usize,
    _marker: PhantomData<*const T>
}

impl<T> MaybeNonDropPointer <T> {
    fn new(ptr: *mut T, drop_behaviour: DropBehaviour) -> MaybeNonDropPointer<T> {
        match drop_behaviour {
            DropBehaviour::ShouldDrop => MaybeNonDropPointer {
                raw_ptr: (ptr as usize) | 1,
                _marker: PhantomData
            },
            DropBehaviour::NoDrop => MaybeNonDropPointer {
                raw_ptr: (ptr as usize) & (!1 as usize),
                _marker: PhantomData
            }
        } 
    }

    fn ptr(&self) -> *const T {
        unsafe {
            let untagged = self.raw_ptr & (!1 as usize);
            transmute(untagged)
        }
    }

    fn mut_ptr(&mut self) -> *mut T {
        unsafe {
            let untagged = self.raw_ptr & (!1 as usize);
            transmute(untagged)
        }
    }

    fn should_drop(&self) -> bool {
        (self.raw_ptr as usize) & 1 == 1
    }
}

trait Allocator {
    fn allocate<T>(cap: usize) -> *mut T;
    unsafe fn deallocate<T>(ptr: *mut T, cap: usize);
}

struct DefaultHeap {}

impl Allocator for DefaultHeap {
    fn allocate<T>(cap: usize) -> *mut T {
        let mut vec = Vec::<T>::with_capacity(cap);
        let ptr = vec.as_mut_ptr();
        mem::forget(vec);

        ptr
    }

    unsafe fn deallocate<T>(ptr: *mut T, cap: usize) {
        let _will_be_dropped = Vec::from_raw_parts(ptr, 0, cap);
    }
}

struct MaybeNonDropVec <T, A: Allocator = DefaultHeap> {
    ptr: MaybeNonDropPointer<T>,
    len: usize,
    cap: usize,
    _alloc: PhantomData<A>
}

impl<T, A: Allocator> MaybeNonDropVec<T, A> {
    pub fn with_capacity(cap: usize) -> MaybeNonDropVec<T, A> {
        MaybeNonDropVec {
            ptr: MaybeNonDropPointer::new(A::allocate::<T>(cap), DropBehaviour::ShouldDrop),
            len: 0,
            cap: cap,
            _alloc: PhantomData
        }
    }

    pub fn from_persistent(ptr: *mut T, len: usize, cap: usize) -> MaybeNonDropVec<T, A> {
        MaybeNonDropVec {
            ptr: MaybeNonDropPointer::new(ptr, DropBehaviour::NoDrop),
            len: len,
            cap: cap,
            _alloc: PhantomData
        }
    }

    fn maybe_drop(&mut self) {
        if self.ptr.should_drop() {
            unsafe {
                ptr::drop_in_place(&mut self[..]);
                A::deallocate(self.ptr.mut_ptr(), self.cap);
            }
        }
    }

    fn double_buf(&mut self) {
        let mut vec = Vec::<T>::with_capacity(self.cap * 2);
        let new_ptr = vec.as_mut_ptr();

        unsafe {
            ptr::copy_nonoverlapping(self.ptr.ptr(), new_ptr, self.len);
        }
        self.maybe_drop();
        self.ptr = MaybeNonDropPointer::new(new_ptr, DropBehaviour::ShouldDrop);
    }

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
}

impl<T, A: Allocator> Drop for MaybeNonDropVec<T, A> {
    fn drop(&mut self) {
        self.maybe_drop();
    }
}

impl<T, A: Allocator> Deref for MaybeNonDropVec<T, A> {
    type Target = [T];

    fn deref(&self) -> &[T] {
        unsafe {
            std::slice::from_raw_parts(self.ptr.ptr(), self.len)
        }
    }
}

impl<T, A: Allocator> DerefMut for MaybeNonDropVec<T, A> {
    fn deref_mut(&mut self) -> &mut [T] {
        unsafe {
            std::slice::from_raw_parts_mut(self.ptr.mut_ptr(), self.len)
        }
    }
}

struct InlineVecPointer<T> {
    offset: u32,
    len: u16,
    cap: u16,
    _marker: PhantomData<T>
}

fn main () {

}