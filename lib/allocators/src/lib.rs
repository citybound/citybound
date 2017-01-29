//! This tiny crate defines a simple allocator interface.
//!
//! It is used by `Compact` types in the `compact` crate to allocate space
//! when decompacting from their compact form.

#![warn(missing_docs)]
#![feature(plugin)]
#![plugin(clippy)]
use std::mem;

/// A trait for all allocators that collections can be generic about
pub trait Allocator {
    /// Allocate enough memory to store `capacity` of `T`
    fn allocate<T>(capacity: usize) -> *mut T;
    /// Free previously allocated memory from pointer.
    ///
    /// Undefined behaviour when passed something else than a pointer
    /// that was created in a call of `allocate`, or when passing a differing `capacity`
    unsafe fn deallocate<T>(ptr: *mut T, capacity: usize);
}

/// An implementation of `Allocator` that allocates using the default heap allocator
///
/// (Uses `Vec::with_capacity internally`)
pub struct DefaultHeap {}

impl Allocator for DefaultHeap {
    fn allocate<T>(capacity: usize) -> *mut T {
        let mut vec = Vec::<T>::with_capacity(capacity);
        let ptr = vec.as_mut_ptr();
        mem::forget(vec);

        ptr
    }

    unsafe fn deallocate<T>(ptr: *mut T, capacity: usize) {
        let _will_be_dropped = Vec::from_raw_parts(ptr, 0, capacity);
    }
}
