use std::mem;
use std::mem::transmute;

pub trait Compact: Sized + Clone {
    /// Is the object's dynamic part compact
    fn is_still_compact(&self) -> bool;

    /// The size of the dynamic part
    fn dynamic_size_bytes(&self) -> usize;

    /// Total size
    fn total_size_bytes(&self) -> usize {
        self.dynamic_size_bytes() + mem::size_of::<Self>()
    }

    /// Move static+dynamic+heap part of `source` to the static+dynamic part of `self`
    unsafe fn compact_from(&mut self, source: &Self, new_dynamic_part: *mut u8);

    /// Get pointer to the start of the dynamic part
    unsafe fn behind(&mut self) -> *mut u8 {
        let behind_self = (self as *mut Self).offset(1);
        transmute(behind_self)
    }

    /// Compact the dynamic+heap part of the `source` into the dynamic part
    unsafe fn compact_behind_from(&mut self, source: &Self) {
        let behind_self = Self::behind(self);
        self.compact_from(source, behind_self)
    }

    // caller has to make sure that self will not be dropped!
    /// Leave only the static+heap part, no dynamic part
    unsafe fn decompact(&self) -> Self;
}

/// Default implementation for sized types
impl<T: Copy> Compact for T {
    fn is_still_compact(&self) -> bool {
        true
    }
    fn dynamic_size_bytes(&self) -> usize {
        0
    }
    unsafe fn compact_from(&mut self, source: &Self, _new_dynamic_part: *mut u8) {
        *self = *source;
    }
    unsafe fn decompact(&self) -> Self {
        *self
    }
}

// TODO: figure out why this doesn't work
// impl<A: Compact, B: Compact> Compact for (A, B) {
//     fn is_still_compact(&self) -> bool {
//         self.0.is_still_compact() && self.1.is_still_compact()
//     }
//     fn dynamic_size_bytes(&self) -> usize {
//         self.0.dynamic_size_bytes() + self.1.dynamic_size_bytes()
//     }
//     unsafe fn compact_from(&mut self, source: &Self, new_dynamic_part: *mut u8) {
//         self.0.compact_from(&source.0, new_dynamic_part);
//         self.1.compact_from(&source.1, new_dynamic_part);
//     }
// }
