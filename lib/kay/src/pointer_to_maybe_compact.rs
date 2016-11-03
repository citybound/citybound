enum Inner<T> {
    Free(*mut T),
    Compact(isize),
    Uninitialized
}

pub struct PointerToMaybeCompact <T> {
    inner: Inner<T>
}

impl<T> Default for PointerToMaybeCompact<T> {
    fn default() -> PointerToMaybeCompact<T> {
        PointerToMaybeCompact {
            inner: Inner::Uninitialized
        }
    }
}

impl<T> PointerToMaybeCompact <T> {
    pub fn new_free(ptr: *mut T) -> Self {
        PointerToMaybeCompact{
            inner: Inner::Free(ptr)
        }
    }

    pub fn set_to_free(&mut self, ptr: *mut T) {
        self.inner = Inner::Free(ptr)
    } 

    pub fn set_to_compact(&mut self, ptr: *mut T) {
        self.inner = Inner::Compact(ptr as isize - self as *const Self as isize);
    }

    pub unsafe fn ptr(&self) -> *const T {
        match self.inner {
            Inner::Free(ptr) => ptr,
            Inner::Compact(offset) => (self as *const Self as *const u8).offset(offset) as *const T,
            Inner::Uninitialized => ::std::ptr::null()
        }
    }

    pub unsafe fn mut_ptr(&mut self) -> *mut T {
        match self.inner {
            Inner::Free(ptr) => ptr,
            Inner::Compact(offset) => (self as *mut Self as *mut u8).offset(offset) as *mut T,
            Inner::Uninitialized => ::std::ptr::null_mut()
        }
    }

    pub fn is_compact(&self) -> bool {
        match self.inner {
            Inner::Free(_) => false,
            Inner::Compact(_) | Inner::Uninitialized => true
        }
    }
}