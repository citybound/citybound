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
        MaybeNonDropPointer{
            raw_ptr: match drop_behaviour{
                DropBehaviour::ShouldDrop => (ptr as usize) | 1,
                DropBehaviour::NoDrop => (ptr as usize) & (!1 as usize)
            },
            _marker: PhantomData
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

struct BackedVec <T, A: Allocator = DefaultHeap> {
    ptr: MaybeNonDropPointer<T>,
    len: usize,
    cap: usize,
    _alloc: PhantomData<A>
}

impl<T, A: Allocator> BackedVec<T, A> {
    pub fn with_capacity(cap: usize) -> BackedVec<T, A> {
        BackedVec {
            ptr: MaybeNonDropPointer::new(A::allocate::<T>(cap), DropBehaviour::ShouldDrop),
            len: 0,
            cap: cap,
            _alloc: PhantomData
        }
    }

    pub fn from_backing(ptr: *mut T, len: usize, cap: usize) -> BackedVec<T, A> {
        BackedVec {
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

impl<T, A: Allocator> Drop for BackedVec<T, A> {
    fn drop(&mut self) {
        self.maybe_drop();
    }
}

impl<T, A: Allocator> Deref for BackedVec<T, A> {
    type Target = [T];

    fn deref(&self) -> &[T] {
        unsafe {
            std::slice::from_raw_parts(self.ptr.ptr(), self.len)
        }
    }
}

impl<T, A: Allocator> DerefMut for BackedVec<T, A> {
    fn deref_mut(&mut self) -> &mut [T] {
        unsafe {
            std::slice::from_raw_parts_mut(self.ptr.mut_ptr(), self.len)
        }
    }
}

struct GrowablePointer<T> {
    offset: u32,
    len_bytes: u16,
    cap_bytes: u16,
    _marker: PhantomData<T>
}

trait GrowablePointerToValue<T> {
    fn set_from_new(&mut self, offset: usize, new: &Option<T>, old: &Self);
    unsafe fn get(ptr: *mut u8, lenu8bytes: usize, cap_bytes: usize) -> T;
}

impl<T> GrowablePointerToValue<BackedVec<T>> for GrowablePointer<BackedVec<T>> {
    fn set_from_new(&mut self, offset: usize, new: &Option<BackedVec<T>>, old: &Self) {
        self.offset = offset as u32;
        self.len_bytes = match new {&Some(ref new_vec) => (new_vec.len * mem::size_of::<T>()) as u16, &None => old.len_bytes};
        self.cap_bytes = match new {&Some(ref new_vec) => (new_vec.cap * mem::size_of::<T>()) as u16, &None => old.cap_bytes};
    }

    unsafe fn get(ptr: *mut u8, len_bytes: usize, cap_bytes: usize) -> BackedVec<T> {
        BackedVec::from_backing(mem::transmute(ptr), len_bytes / mem::size_of::<T>(), cap_bytes / mem::size_of::<T>())
    }
}

impl<T> Clone for GrowablePointer<T> {
    fn clone(&self) -> GrowablePointer<T> {
        match *self {
            GrowablePointer {
                offset: offset,
                len_bytes: len_bytes,
                cap_bytes: cap_bytes,
                _marker: _
            } => GrowablePointer {
                offset: offset,
                len_bytes: len_bytes,
                cap_bytes: cap_bytes,
                _marker: PhantomData
            }
        }
    }

}

macro_rules! actor {
    (struct $name:ident $fields:tt growables $ifields:tt $update_impl:item) => {
        actor_combined_struct!($name ; $fields ; $ifields);
        actor_update_fields_struct!(GrowablesUpdate ; $ifields);
        actor_impl!($name ; $ifields ; GrowablesUpdate);
        $update_impl
    }
}

macro_rules! actor_combined_struct {
    ($name:ident ; {$($field:ident : $ftype:ty),*} ; {$($ifield:ident: $itype:ty),*}) => {
        #[derive(Clone)]
        struct $name {
            $($field: $ftype),*,
            $($ifield: GrowablePointer<$itype>),*
        }
    }
}

macro_rules! actor_impl {
    ($name:ident ; $ifields:tt ; $update_struct:ident)  => {
        impl $name {
            actor_update_wrapper!($ifields ; $update_struct);
            actor_growables_getters!($ifields);
        }
    }
}

macro_rules! actor_update_fields_struct {
    ($name:ident ; {$($ifield:ident: $itype:ty),*}) => {
        struct $name {
            $($ifield: Option<$itype>),*
        }
    }
}

macro_rules! actor_update_wrapper {
    ($ifields:tt ; $update_struct:ident) => {
        pub fn update_and_resize<F: Fn(usize) -> (*mut u8, usize, usize), G: Fn(usize, usize, usize)>
        (&mut self, get_slot: F, move_to_slot: G) {
            let field_updates : Option<$update_struct> = self.update();
            match field_updates {
                None | Some(actor_update_fields_all_none!($update_struct ; $ifields)) => {return},
                Some(new_fields) => {
                    let mut new_self = self.clone();
                    let mut total_size_requirement = mem::size_of::<Self>();
                    let mut current_offset = mem::size_of::<Self>();

                    actor_ifields_resize_calc!($ifields ; new_fields ; self ; new_self ; total_size_requirement ; current_offset);

                    let (new_slot, new_slot_cap, new_slot_id) = get_slot(total_size_requirement);
                    unsafe {
                        let self_in_new_slot : *mut Self = mem::transmute(new_slot);
                        *self_in_new_slot = new_self.clone();
                    }

                    actor_ifields_copy!($ifields ; new_fields ; self ; new_self ; new_slot);

                    move_to_slot(self.id, new_slot_cap, new_slot_id);
                }
            }
        }
    }
}

macro_rules! actor_update_fields_all_none {
    ($struct_name:ident ; {$($ifield:ident: $vtype:ty),*}) => {
        $struct_name {
            $($ifield: None),*
        }
    }
}

macro_rules! actor_growables_getters {
    ({$($ifield:ident: $vtype:ty),*}) => {
        $(
            pub fn $ifield (&mut self) -> $vtype {
                unsafe {
                    let base_ptr : *mut u8 = transmute(self as *mut Self);
                    GrowablePointer::get(
                        base_ptr.offset(self.$ifield.offset as isize),
                        self.$ifield.len_bytes as usize,
                        self.$ifield.cap_bytes as usize
                    )
                }
            }
        )*
    }
}

macro_rules! actor_ifields_resize_calc {
    ({$($ifield:ident: $vtype:ty),*} ; $new_fields:ident ; $the_self:ident ; $new_self:ident ; $total_size_requirement:ident ; $current_offset:ident) => {
        $(
            {
                $new_self.$ifield.set_from_new($current_offset, &$new_fields.$ifield, &$the_self.$ifield);
                $total_size_requirement += $new_self.$ifield.cap_bytes as usize;
                $current_offset += $new_self.$ifield.cap_bytes as usize;
            }
        )*
    }
}

macro_rules! actor_ifields_copy {
    ({$($ifield:ident: $vtype:ty),*} ; $new_fields:ident ; $the_self:ident ; $new_self:ident ; $new_slot:ident) => {
        $(
            unsafe {
                let old_self_pointer : *const u8 = transmute($the_self as *const Self);
                let src = old_self_pointer.offset($the_self.$ifield.offset as isize);
                let dest = $new_slot.offset($new_self.$ifield.offset as isize);
                ptr::copy_nonoverlapping(src, dest, $new_self.$ifield.len_bytes as usize);
            }
        )*
    }
}

actor! {
    struct Test {
        id: usize,
        a: u32,
        b: u16
    }

    growables {
        x: BackedVec<u8>,
        y: BackedVec<u16>
    }

    impl Test {
        fn update (&mut self) -> Option<GrowablesUpdate> {
            self.a;
            Some(GrowablesUpdate{x: None, y: None})
        }
    }
}

fn main () {
    let mut t : Test;
    t.x();
}