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

    /// Copy the static part of `source` to `dest` and compactly store
    /// the dynamic part of `source` as the new dynamic part of `dest` at `new_dynamic_part`.
    /// This semantically moves source into dest.
    unsafe fn compact(source: *mut Self, dest: *mut Self, new_dynamic_part: *mut u8);

    /// Get a pointer to behind the static part of `self` (commonly used place for the dynamic part)
    unsafe fn behind(ptr: *mut Self) -> *mut u8 {
        transmute(ptr.offset(1))
    }

    /// Like `compact` with `new_dynamic_part` set to `dest.behind()`
    unsafe fn compact_behind(source: *mut Self, dest: *mut Self) {
        let behind_dest = Self::behind(dest);
        Self::compact(source, dest, behind_dest)
    }

    /// Creates a clone of self with the dynamic part guaranteed to be stored freely.
    ///
    /// *Note:* if the dynamic part was already stored freely, the calling environment
    /// has to make sure that old self will not be dropped, as this might lead to a double free!
    ///
    /// This is mostly used internally to correctly implement
    /// `Compact` datastructures that contain `Compact` elements.
    unsafe fn decompact(source: *const Self) -> Self;
}

/// Trivial implementation for fixed-sized, `Copy` types (no dynamic part)
impl<T: Copy> Compact for T {
    default fn is_still_compact(&self) -> bool {
        true
    }
    default fn dynamic_size_bytes(&self) -> usize {
        0
    }
    default unsafe fn compact(source: *mut Self, dest: *mut Self, _new_dynamic_part: *mut u8) {
        *dest = *source
    }

    default unsafe fn decompact(source: *const Self) -> Self {
        *source
    }
}
