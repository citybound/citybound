use std::ops::{Index, IndexMut, Deref};

pub trait MergeGroupsVecLike<T: Sized> : Index<usize, Output=T> + IndexMut<usize> + Deref<Target=[T]> {
    fn is_empty(&self) -> bool;
    fn len(&self) -> usize;
    fn extend_from_slice(&mut self, slice: &[T]);
    fn remove(&mut self, index: usize) -> T;
}

pub trait MergeGroups<T, C1: MergeGroupsVecLike<T> + Clone> : MergeGroupsVecLike<C1> {
    fn merge_groups<F: Fn(
        &MergeGroupsVecLike<T, Output=T, Target=[T]>,
        &MergeGroupsVecLike<T, Output=T, Target=[T]>
    ) -> bool>(&mut self, merge_f: F) {
        let mut old_len = 0;
        while self.len() != old_len {
            old_len = self.len();
            let mut i = 0;
            #[allow(needless_range_loop)]
            while i < self.len() {
                for j in ((i + 1)..self.len()).rev() {
                    if !self[i].is_empty() && !self[j].is_empty() && merge_f(&self[i], &self[j]) {
                        let group_to_merge = self[j].clone();
                        self[i].extend_from_slice(group_to_merge.deref());
                        self.remove(j);
                    }
                }
                i += 1;
            }
        }
    }
}

impl<T: Clone> MergeGroupsVecLike<T> for Vec<T> {
    fn is_empty(&self) -> bool {Vec::is_empty(self)}
    fn len(&self) -> usize {Vec::len(self)}
    fn extend_from_slice(&mut self, slice: &[T]) {Vec::extend_from_slice(self, slice)}
    fn remove(&mut self, index: usize) -> T {Vec::remove(self, index)}
}

impl<T: Clone> MergeGroups<T, Vec<T>> for Vec<Vec<T>> {}