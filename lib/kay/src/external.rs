use compact::Compact;
use std::sync::Arc;

pub struct External<T> {
    ext: Arc<T>,
}

impl<T> External<T> {
    pub fn new(content: T) -> Self {
        External { ext: Arc::new(content) }
    }

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
    unsafe fn compact_to(&mut self, _new_dynamic_part: *mut u8) {}
    unsafe fn decompact(&self) -> Self {
        self.clone()
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