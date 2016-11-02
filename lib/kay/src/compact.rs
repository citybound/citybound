use std::mem;
use std::mem::transmute;

pub trait Compact : Sized {
    fn is_still_compact(&self) -> bool;
    fn dynamic_size_bytes(&self) -> usize;
    fn total_size_bytes(&self) -> usize {
        self.dynamic_size_bytes() + mem::size_of::<Self>()
    }
    unsafe fn compact_from(&mut self, source: &Self, new_dynamic_part: *mut u8);
    unsafe fn behind(&mut self) -> *mut u8 {
        let behind_self = (self as *mut Self).offset(1);
        transmute(behind_self)
    }
    unsafe fn compact_behind_from(&mut self, source: &Self) {
        let behind_self = Self::behind(self);
        self.compact_from(source, behind_self)
    }
}

impl<T: Copy> Compact for T {
    fn is_still_compact(&self) -> bool {true}
    fn dynamic_size_bytes(&self) -> usize {0}
    unsafe fn compact_from(&mut self, source: &Self, _new_dynamic_part: *mut u8) {
        *self = *source;
    }
}

#[macro_export]
macro_rules! derive_compact {
    (struct $name:ident $fields:tt) => {
        echo_struct!($name, $fields);
        derive_compact_impl!($name, $fields);
    };

    (pub struct $name:ident $fields:tt) => {
        echo_pub_struct!($name, $fields);
        derive_compact_impl!($name, $fields);
    }
}

#[macro_export]
macro_rules! derive_compact_impl {
    ($name:ident, $fields:tt) => {
        impl Compact for $name {
            fn is_still_compact(&self) -> bool {
                derive_is_still_compact!(self, $fields)
            }

            fn dynamic_size_bytes(&self) -> usize {
                derive_dynamic_size_bytes!(self, $fields)
            }

            unsafe fn compact_from(&mut self, source: &Self, new_dynamic_part: *mut u8) {
                #![allow(unused_assignments)]
                let mut offset: isize = 0;
                derive_compact_from!(self, source, new_dynamic_part, offset, $fields);
            }
        }
    }
}

#[macro_export]
macro_rules! echo_struct {
    ($name:ident, {$($field:ident: $field_type:ty),*}) => {
        struct $name {
            $($field: $field_type),*
        }
    }
}

#[macro_export]
macro_rules! echo_pub_struct {
    ($name:ident, {$($field:ident: $field_type:ty),*}) => {
        pub struct $name {
            $($field: $field_type),*
        }
    }
}

#[macro_export]
macro_rules! derive_is_still_compact {
    ($the_self:ident, {$($field:ident: $field_type:ty),*}) => {
        $($the_self.$field.is_still_compact())&&*
    }
}

#[macro_export]
macro_rules! derive_dynamic_size_bytes {
    ($the_self:ident, {$($field:ident: $field_type:ty),*}) => {
        $($the_self.$field.dynamic_size_bytes() + )* 0
    }
}

#[macro_export]
macro_rules! derive_compact_from {
    ($the_self:ident, $source:ident, $new_dynamic_part:ident, $offset:ident, {$($field:ident: $field_type:ty),*}) => {
        $(
            $the_self.$field.compact_from(&$source.$field, $new_dynamic_part.offset($offset));
            $offset += $source.$field.dynamic_size_bytes() as isize;
        )*
    }
}