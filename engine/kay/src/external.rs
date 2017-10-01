use compact::Compact;
use std::cell::Cell;

// TODO: make this much more simple and just like a Box once we can move out of messages!

/// An owning reference to local state outside the actor system that can safely be embedded in
/// actor states and passed in messages, as long as they stay on one machine.
pub struct External<T> {
    maybe_owned: Cell<Option<Box<T>>>,
}

impl<T> External<T> {
    /// Allocate `content` on the heap and create a sharable `External` reference to it
    pub fn new(content: T) -> Self {
        External { maybe_owned: Cell::new(Some(Box::new(content))) }
    }

    /// To interface with traditional owned boxes
    pub fn from_box(content: Box<T>) -> Self {
        External { maybe_owned: Cell::new(Some(content)) }
    }

    /// Like `clone`, just to make the danger more clear
    pub fn steal(&self) -> Self {
        self.clone()
    }

    /// To interface with traditional owned boxes
    pub fn into_box(self) -> Box<T> {
        self.maybe_owned.into_inner().expect(
            "Tried to get Box from already taken external",
        )
    }
}

// TODO: this is _really_ screwy, see above
impl<T> Clone for External<T> {
    fn clone(&self) -> Self {
        External {
            maybe_owned: Cell::new(Some(self.maybe_owned.take().expect(
                "Tried to clone already taken external",
            ))),
        }
    }
}

impl<T> ::std::ops::Deref for External<T> {
    type Target = T;

    fn deref(&self) -> &T {
        let option_ref = unsafe { &(*self.maybe_owned.as_ptr()) };
        &**option_ref.as_ref().expect(
            "Tried to deref already taken external",
        )
    }
}

impl<T> ::std::ops::DerefMut for External<T> {
    fn deref_mut(&mut self) -> &mut T {
        &mut *self.maybe_owned.get_mut().as_mut().expect(
            "Tried to mut deref already taken external",
        )
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
