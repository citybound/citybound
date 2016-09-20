use std::mem;

pub trait Allocator {
    fn allocate<T>(cap: usize) -> *mut T;
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