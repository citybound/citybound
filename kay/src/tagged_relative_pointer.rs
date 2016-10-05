use std::marker::PhantomData;
use std::mem::transmute;

// TODO: potential optimization with actual bit-tagged pointer
// but tricky, since the offset can be negative (two-complement)
pub struct TaggedRelativePointer <T> {
    offset: i32,
    tagged: bool,
    _marker: PhantomData<*const T>
}

impl<T> Default for TaggedRelativePointer<T> {
    fn default() -> TaggedRelativePointer<T> {
        TaggedRelativePointer {
            offset: 0,
            tagged: false,
            _marker: PhantomData
        }
    }
}

impl<T> TaggedRelativePointer <T> {
    pub fn null(tagged: bool) -> TaggedRelativePointer<T> {
        TaggedRelativePointer{
            offset: 0,
            tagged: tagged,
            _marker: PhantomData
        }
    }

    pub fn set(&mut self, ptr: *mut T, tagged: bool) {
        self.offset = ((ptr as isize) - ((self as *const Self) as isize)) as i32;
        self.tagged = tagged
    }

    pub unsafe fn ptr(&self) -> *const T {
        transmute::<*const u8, *const T>(
                transmute::<*const Self, *const u8>(self as *const Self)
                    .offset(self.offset as isize))
    }

    pub unsafe fn mut_ptr(&mut self) -> *mut T {
        transmute::<*mut u8, *mut T>(
                transmute::<*const Self, *mut u8>(self as *mut Self)
                    .offset(self.offset as isize))
    }

    pub fn is_tagged(&self) -> bool {
        self.tagged
    }
}