use std::mem;
use std::mem::transmute;

pub trait Compact : Sized + Clone {
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