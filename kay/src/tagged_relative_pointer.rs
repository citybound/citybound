use std::marker::PhantomData;
use std::mem::transmute;

pub struct TaggedRelativePointer <T> {
    offset: i32,
    _marker: PhantomData<*const T>
}

impl<T> Default for TaggedRelativePointer<T> {
    fn default() -> TaggedRelativePointer<T> {
        TaggedRelativePointer {
            offset: 0,
            _marker: PhantomData
        }
    }
}

const TAG_BIT : i32 = 0b100_0000_0000_0000i32;

impl<T> TaggedRelativePointer <T> {
    pub fn null(tagged: bool) -> TaggedRelativePointer<T> {
        TaggedRelativePointer{
            offset: match tagged {false => 0, true => TAG_BIT},
            _marker: PhantomData
        }
    }

    pub fn set(&mut self, ptr: *mut T, tagged: bool) {
        let mut offset : i32 = ((ptr as isize) - ((self as *const Self) as isize)) as i32;
        if tagged {offset |= TAG_BIT}
        self.offset = offset;
    }

    pub unsafe fn ptr(&self) -> *const T {
        transmute::<*const u8, *const T>(
                transmute::<*const Self, *const u8>(self as *const Self)
                    .offset((self.offset & !TAG_BIT) as isize))
    }

    pub unsafe fn mut_ptr(&mut self) -> *mut T {
        transmute::<*mut u8, *mut T>(
                transmute::<*const Self, *mut u8>(self as *mut Self)
                    .offset((self.offset & !TAG_BIT) as isize))
    }

    pub fn is_tagged(&self) -> bool {
        self.offset & TAG_BIT == TAG_BIT
    }
}