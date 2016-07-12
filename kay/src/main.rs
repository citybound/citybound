#![allow(dead_code)]
use std::mem;
use std::mem::transmute;
use std::ptr;
use std::marker::PhantomData;
use std::ops::{Deref, DerefMut};

struct TaggedRelativePointer <T> {
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

const TAG_BIT : i32 = 0xb100_0000_0000_0000i32;

impl<T> TaggedRelativePointer <T> {
    fn null(tagged: bool) -> TaggedRelativePointer<T> {
        TaggedRelativePointer{
            offset: match tagged {false => 0, true => TAG_BIT},
            _marker: PhantomData
        }
    }

    fn set(&mut self, ptr: *mut T, tagged: bool) -> TaggedRelativePointer<T> {
        let mut offset : i32 = ((ptr as isize) - ((self as *const Self) as isize)) as i32;
        if tagged {offset |= TAG_BIT}
        TaggedRelativePointer{
            offset: offset,
            _marker: PhantomData
        }
    }

    unsafe fn ptr(&self) -> *const T {
        transmute::<*const u8, *const T>(
                transmute::<*const Self, *const u8>(self as *const Self)
                    .offset((self.offset & !TAG_BIT) as isize))
    }

    unsafe fn mut_ptr(&mut self) -> *mut T {
        transmute::<*mut u8, *mut T>(
                transmute::<*const Self, *mut u8>(self as *mut Self)
                    .offset((self.offset & !TAG_BIT) as isize))
    }

    fn is_tagged(&self) -> bool {
        self.offset & TAG_BIT == TAG_BIT
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

struct EmbeddedVec <T, A: Allocator = DefaultHeap> {
    ptr: TaggedRelativePointer<T>,
    len: usize,
    cap: usize,
    _alloc: PhantomData<A>
}

const FREE : bool = true;
const EMBEDDED : bool = false;

impl<T, A: Allocator> EmbeddedVec<T, A> {
    pub fn new() -> EmbeddedVec<T, A> {
        EmbeddedVec {
            ptr: TaggedRelativePointer::null(EMBEDDED),
            len: 0,
            cap: 0,
            _alloc: PhantomData
        }
    }

    pub fn with_capacity(cap: usize) -> EmbeddedVec<T, A> {
        let mut vec = EmbeddedVec {
            ptr: TaggedRelativePointer::default(),
            len: 0,
            cap: cap,
            _alloc: PhantomData
        };

        vec.ptr.set(A::allocate::<T>(cap), FREE);
        vec
    }

    pub fn from_backing(ptr: *mut T, len: usize, cap: usize) -> EmbeddedVec<T, A> {
        let mut vec = EmbeddedVec {
            ptr: TaggedRelativePointer::default(),
            len: len,
            cap: cap,
            _alloc: PhantomData
        };

        vec.ptr.set(ptr, EMBEDDED);
        vec
    }

    fn maybe_drop(&mut self) {
        if self.ptr.is_tagged() == FREE {
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
        self.ptr.set(new_ptr, FREE);
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

impl<T, A: Allocator> Drop for EmbeddedVec<T, A> {
    fn drop(&mut self) {
        self.maybe_drop();
    }
}

impl<T, A: Allocator> Deref for EmbeddedVec<T, A> {
    type Target = [T];

    fn deref(&self) -> &[T] {
        unsafe {
            std::slice::from_raw_parts(self.ptr.ptr(), self.len)
        }
    }
}

impl<T, A: Allocator> DerefMut for EmbeddedVec<T, A> {
    fn deref_mut(&mut self) -> &mut [T] {
        unsafe {
            std::slice::from_raw_parts_mut(self.ptr.mut_ptr(), self.len)
        }
    }
}

trait Embedded {
    fn is_still_embedded(&self) -> bool;
    fn data_cap_in_bytes(&self) -> usize;
    fn data_len_in_bytes(&self) -> usize;
    unsafe fn data_ptr(&self) -> *const u8;
    unsafe fn re_embed(&mut self, new_embedded_data: *mut u8);
}

impl<T, A: Allocator> Embedded for EmbeddedVec<T, A> {
    fn is_still_embedded(&self) -> bool {
        self.ptr.is_tagged() == EMBEDDED
    }

    fn data_cap_in_bytes(&self) -> usize {
        self.cap * mem::size_of::<T>()
    }

    fn data_len_in_bytes(&self) -> usize {
        self.len * mem::size_of::<T>()
    }

    unsafe fn data_ptr(&self) -> *const u8 {
        transmute(self.ptr.ptr())
    }

    unsafe fn re_embed(&mut self, new_embedded_data: *mut u8) {
        self.ptr.set(transmute(new_embedded_data), EMBEDDED);
    }
}

macro_rules! trivially_embedded {
    ($($trivial_type:ty),*) => {
        $(
            impl Embedded for $trivial_type {
                fn is_still_embedded(&self) -> bool {true}
                fn data_cap_in_bytes(&self) -> usize {0}
                fn data_len_in_bytes(&self) -> usize {0}
                unsafe fn data_ptr(&self) -> *const u8 {ptr::null()}
                unsafe fn re_embed(&mut self, _new_embedded_data: *mut u8) {}
            }
        )*
    }
}

trivially_embedded!(usize, u32, u16);

macro_rules! derive_embeddable {
    (struct $name:ident $fields:tt) => {
        echo_struct!($name, $fields);

        impl Embedded for $name {
            fn is_still_embedded(&self) -> bool {
                derive_is_still_embedded!(self, $fields)
            }

            fn data_cap_in_bytes(&self) -> usize {
                derive_data_cap_in_bytes!(self, $fields)
            }

            fn data_len_in_bytes(&self) -> usize {
                self.data_cap_in_bytes()
            }

            unsafe fn data_ptr(&self) -> *const u8 {
                transmute::<*const Self, *const u8>(self as *const Self)
                    .offset(mem::size_of::<Self>() as isize)
            }

            unsafe fn re_embed(&mut self, new_embedded_data: *mut u8) {
                let mut offset: isize = 0;
                derive_re_embed!(self, new_embedded_data, offset, $fields);
            }
        }
    }
}

macro_rules! echo_struct {
    ($name:ident, {$($field:ident: $field_type:ty),*}) => {
        struct $name {
            $($field: $field_type),*
        }
    }
}

macro_rules! derive_is_still_embedded {
    ($the_self:ident, {$($field:ident: $field_type:ty),*}) => {
        $($the_self.$field.is_still_embedded())&&*
    }
}

macro_rules! derive_data_cap_in_bytes {
    ($the_self:ident, {$($field:ident: $field_type:ty),*}) => {
        $($the_self.$field.data_cap_in_bytes() + )* 0
    }
}

macro_rules! derive_re_embed {
    ($the_self:ident, $new_embedded_data:ident, $offset:ident, {$($field:ident: $field_type:ty),*}) => {
        $(
            $the_self.$field.re_embed($new_embedded_data.offset($offset));
            $offset += $the_self.$field.data_cap_in_bytes() as isize;
        )*
    }
}

derive_embeddable!{
    struct Test {
        id: usize,
        a: u32,
        b: u16,
        x: EmbeddedVec<u8>,
        y: EmbeddedVec<u16>
    }
}

fn main () {
    let mut t = Test {
        id: 0,
        a: 1,
        b: 2,
        x: EmbeddedVec::new(),
        y: EmbeddedVec::new()
    };
    t.x.push(5u8);
    println!("{}", t.x.len);
}