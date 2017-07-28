use compact::Compact;
use std::sync::Arc;

/// A reference to local state outside the actor system.
/// Can safely be embedded in actor states and messages, as long as they stay on one machine.
/// Reference counting and deallocation is handled, since it uses an `Arc` internally
pub struct External<T> {
    ext: Arc<T>,
}

impl<T> External<T> {
    /// Allocate `content` on the heap and create a sharable `External` reference to it
    pub fn new(content: T) -> Self {
        External { ext: Arc::new(content) }
    }

    /// Try to get mutable (exclusive) access to the referenced resource.
    /// Just like `Arc::get_mut`, this only succeeds
    /// if the caller is the only holder of a reference.
    pub fn get_mut(external: &mut Self) -> Option<&mut T> {
        Arc::get_mut(&mut external.ext)
    }
}

impl<T> Clone for External<T> {
    fn clone(&self) -> Self {
        External { ext: self.ext.clone() }
    }
}

impl<T> Compact for External<T> {
    fn is_still_compact(&self) -> bool {
        true
    }
    fn dynamic_size_bytes(&self) -> usize {
        0
    }
    unsafe fn compact(source: *mut Self, dest: *mut Self, _new_dynamic_part: *mut u8) {
        ::std::ptr::copy_nonoverlapping(source, dest, 1)
    }
    unsafe fn decompact(source: *const Self) -> Self {
        ::std::ptr::read(source)
    }
}

// impl<T> Drop for External<T> {
//     fn drop(&mut self) {
//         panic!("Droppy drop")
//     }
// }

impl<T> ::std::ops::Deref for External<T> {
    type Target = Arc<T>;

    fn deref(&self) -> &Arc<T> {
        &self.ext
    }
}

impl<T> ::std::ops::DerefMut for External<T> {
    fn deref_mut(&mut self) -> &mut Arc<T> {
        &mut self.ext
    }
}
