use std::mem;
use std::mem::transmute;

/// A trait for objects with a statically-sized part and a potential dynamically-sized part
/// that can be stored both compactly in consecutive memory or freely on the heap
pub trait Compact: Sized + Clone {
    /// Is the object's dynamic part stored compactly?
    fn is_still_compact(&self) -> bool;

    /// Size of the dynamic part in bytes
    fn dynamic_size_bytes(&self) -> usize;

    /// Total size of the object (static part + dynamic part)
    fn total_size_bytes(&self) -> usize {
        self.dynamic_size_bytes() + mem::size_of::<Self>()
    }

    /// Compactly store the dynamic part of `self` at `new_dynamic_part`.
    unsafe fn compact_to(&mut self, new_dynamic_part: *mut u8);

    /// Get a pointer to behind the static part of `self` (commonly used place for the dynamic part)
    unsafe fn behind(&mut self) -> *mut u8 {
        let behind_self = (self as *mut Self).offset(1);
        transmute(behind_self)
    }

    /// Like `compact_from` with `new_dynamic_part` set to `self.behind()`
    unsafe fn compact_behind(&mut self) {
        let behind_self = Self::behind(self);
        self.compact_to(behind_self)
    }

    /// Creates a clone of self with the dynamic part guaranteed to be stored freely.
    ///
    /// *Note:* if the dynamic part was already stored freely, the calling environment
    /// has to make sure that old self will not be dropped, as this might lead to a double free!
    ///
    /// This is mostly used internally to correctly implement
    /// `Compact` datastructures that contain `Compact` elements.
    unsafe fn decompact(&self) -> Self;
}

/// Trivial implementation for fixed-sized, `Copy` types (no dynamic part)
impl<T: Copy> Compact for T {
    fn is_still_compact(&self) -> bool {
        true
    }
    fn dynamic_size_bytes(&self) -> usize {
        0
    }
    unsafe fn compact_to(&mut self, _new_dynamic_part: *mut u8) {}

    unsafe fn decompact(&self) -> Self {
        *self
    }
}