use std;
/// Specifies the 3 states that the pointer can be in:
/// 1. Free: On the heap - Stores a pointer
/// 2. Compact: On the dynamic part - Stores an offset
/// 3. Null
enum Inner<T> {
    Free(*mut T),
    Compact(isize),
    Uninitialized,
}

/// See Inner
pub struct PointerToMaybeCompact<T> {
    inner: Inner<T>,
}

impl<T> Default for PointerToMaybeCompact<T> {
    fn default() -> PointerToMaybeCompact<T> {
        PointerToMaybeCompact { inner: Inner::Uninitialized }
    }
}

impl<T> std::fmt::Debug for PointerToMaybeCompact<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "Ptr {:?}", self.to_string())
    }
}

impl<T> PointerToMaybeCompact<T> {
    /// Create a new pointer which is initialized to point on the heap
    pub fn new_free(ptr: *mut T) -> Self {
        PointerToMaybeCompact { inner: Inner::Free(ptr) }
    }

    /// Set the pointer to point on the heap
    pub fn set_to_free(&mut self, ptr: *mut T) {
        self.inner = Inner::Free(ptr)
    }

    /// Set the pointer to point on the dynamic part of the data structure
    pub fn set_to_compact(&mut self, ptr: *mut T) {
        self.inner = Inner::Compact(ptr as isize - self as *const Self as isize);
    }

    /// Get a raw pointer to wherever it is pointing
    pub unsafe fn ptr(&self) -> *const T {
        match self.inner {
            Inner::Free(ptr) => ptr,
            Inner::Compact(offset) => (self as *const Self as *const u8).offset(offset) as *const T,
            Inner::Uninitialized => ::std::ptr::null(),
        }
    }

    /// Get a mut pointer to wherever it is pointing
    pub unsafe fn mut_ptr(&mut self) -> *mut T {
        match self.inner {
            Inner::Free(ptr) => ptr,
            Inner::Compact(offset) => (self as *mut Self as *mut u8).offset(offset) as *mut T,
            Inner::Uninitialized => ::std::ptr::null_mut(),
        }
    }

    /// Check to see if pointer is on the dynamic part of the data structure
    pub fn is_compact(&self) -> bool {
        match self.inner {
            Inner::Free(_) => false,
            Inner::Compact(_) |
            Inner::Uninitialized => true,
        }
    }

    pub fn to_string(&self) -> String {
        match self.inner {
            Inner::Free(p) => format!("Free {:p}", p),
            Inner::Compact(i) => format!("Compact {:?}", i),
            Inner::Uninitialized => String::from("uninitialized"),
        }
    }
}

impl<T: Default> PointerToMaybeCompact<T> {
    pub fn initialize_with_default(&self, i: isize) {
        match self.inner {
            Inner::Uninitialized => (),
            Inner::Compact(i) => (),
            Inner::Free(p) => (unsafe { std::ptr::write(p.offset(i), T::default()) }),
        }
    }
}
