use super::Construction;
use compact::Compact;

impl Compact for Construction {
    fn is_still_compact(&self) -> bool {
        true
    }

    fn dynamic_size_bytes(&self) -> usize {
        0
    }

    unsafe fn compact(source: *mut Self, dest: *mut Self, new_dynamic_part: *mut u8) {
        ::std::ptr::copy(source, dest, 1);
    }

    unsafe fn decompact(source: *const Self) -> Self {
        ::std::ptr::read(source)
    }
}
