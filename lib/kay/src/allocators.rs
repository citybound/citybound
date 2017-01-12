use std::mem;

/// Something that can allocate memory
pub trait Allocator {
    /// Allocate enough memory to store `cap` of `T`
    fn allocate<T>(cap: usize) -> *mut T;
    /// Free memory from pointer
    unsafe fn deallocate<T>(ptr: *mut T, cap: usize);
}

pub struct DefaultHeap {}

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
