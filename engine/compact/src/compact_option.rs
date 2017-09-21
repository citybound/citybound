use super::compact::Compact;

/// A wrapper to make an `Option` of a nontrivial `Compact` possible.
/// Unfortunately, we can't blanket-`impl` that, since that overlaps
/// (for the compiler) with the `impl` for trivial `Copy` types...
#[derive(Clone)]
pub struct CompactOption<T: Compact + Clone>(pub Option<T>);

impl<T: Compact + Clone> ::std::ops::Deref for CompactOption<T> {
    type Target = Option<T>;

    fn deref(&self) -> &Option<T> {
        &self.0
    }
}

impl<T: Compact + Clone> ::std::ops::DerefMut for CompactOption<T> {
    fn deref_mut(&mut self) -> &mut Option<T> {
        &mut self.0
    }
}

impl<T: Clone + Compact> Compact for CompactOption<T> {
    fn is_still_compact(&self) -> bool {
        self.0.as_ref().map(|t| t.is_still_compact()).unwrap_or(
            true,
        )
    }

    fn dynamic_size_bytes(&self) -> usize {
        self.0.as_ref().map(|t| t.dynamic_size_bytes()).unwrap_or(0)
    }

    unsafe fn compact(source: *mut Self, dest: *mut Self, new_dynamic_part: *mut u8) {
        if let CompactOption(Some(ref mut s)) = *source {
            ::std::ptr::write(dest, CompactOption(Some(::std::mem::uninitialized())));
            if let CompactOption(Some(ref mut d)) = *dest {
                Compact::compact(s, d, new_dynamic_part);
            } else {
                unreachable!()
            }
        } else {
            ::std::ptr::write(dest, CompactOption(None));
        }
    }

    unsafe fn decompact(source: *const Self) -> Self {
        if let CompactOption(Some(ref s)) = *source {
            CompactOption(Some(Compact::decompact(s)))
        } else {
            CompactOption(None)
        }
    }
}

#[test]
fn basic_option() {
    use super::compact_vec::CompactVec;
    use super::allocators::{Allocator, DefaultHeap};
    let mut option: CompactOption<CompactVec<u32>> = CompactOption(Some(CompactVec::new()));

    if let Some(ref mut list) = *option {
        list.push(1);
        list.push(2);
        list.push(3);
        assert_eq!(&[1, 2, 3], &**list);
    } else {
        unreachable!()
    }


    let bytes = option.total_size_bytes();
    let storage = DefaultHeap::allocate(bytes);

    unsafe {
        Compact::compact_behind(&mut option, storage as *mut CompactOption<CompactVec<u32>>);
        ::std::mem::forget(option);
        if let Some(ref list) = **(storage as *mut CompactOption<CompactVec<u32>>) {
            assert_eq!(&[1, 2, 3], &**list);
        } else {
            unreachable!()
        }
        println!("before decompact!");
        if let Some(ref list) = *Compact::decompact(
            storage as *mut CompactOption<CompactVec<u32>>,
        )
        {
            assert_eq!(&[1, 2, 3], &**list);
        } else {
            unreachable!()
        }
        DefaultHeap::deallocate(storage, bytes);
    }
}