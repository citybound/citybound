use roaring::RoaringBitmap;

pub struct DisjointSets<T> {
    elements: Vec<T>,
    parent_indices: Vec<usize>,
    ranks: Vec<usize>,
    is_sorted: bool,
}

impl<T> DisjointSets<T> {
    pub fn from_individuals(individuals: Vec<T>) -> Self {
        DisjointSets {
            parent_indices: (0..individuals.len()).into_iter().collect(),
            ranks: vec![0; individuals.len()],
            elements: individuals,
            is_sorted: true,
        }
    }

    fn find_root(&mut self, idx: usize) -> usize {
        if self.parent_indices[idx] != idx {
            let parent = self.parent_indices[idx];
            self.parent_indices[idx] = self.find_root(parent);
        }
        self.parent_indices[idx]
    }

    fn union(&mut self, idx_a: usize, idx_b: usize) {
        let root_a = self.find_root(idx_a);
        let root_b = self.find_root(idx_b);

        if root_a != root_b {
            if self.ranks[root_a] < self.ranks[root_b] {
                self.parent_indices[root_a] = root_b;
            } else if self.ranks[root_a] > self.ranks[root_b] {
                self.parent_indices[root_b] = root_a;
            } else {
                self.parent_indices[root_b] = root_a;
                self.ranks[root_a] += 1;
            }
        }

        self.is_sorted = false;
    }

    #[inline(never)]
    pub fn union_all_with<F: Fn(&T, &T) -> bool>(&mut self, should_union: F) {
        let len = self.elements.len();
        for idx_a in 0..len {
            for idx_b in (idx_a + 1)..len {
                if should_union(&self.elements[idx_a], &self.elements[idx_b]) {
                    self.union(idx_a, idx_b);
                }
            }
        }
    }

    pub fn union_all_with_accelerator<
        Acc,
        FAdd: Fn(&T, usize, &mut Acc),
        FPairs: Fn(&Acc) -> &Vec<(usize, RoaringBitmap)>,
        F: Fn(&T, &T) -> bool,
    >(
        &mut self,
        initial_accelerator: Acc,
        add: FAdd,
        pairs: FPairs,
        should_union: F,
    ) {
        let mut accelerator = initial_accelerator;
        for (i, element) in self.elements.iter().enumerate() {
            add(element, i, &mut accelerator);
        }

        // TODO: use fact that pairs are commutative
        for &(idx_a, ref idx_b_bmap) in pairs(&accelerator) {
            for idx_b in idx_b_bmap.iter() {
                if should_union(&self.elements[idx_a], &self.elements[idx_b as usize]) {
                    self.union(idx_a, idx_b as usize);
                }
            }
        }
    }

    #[cfg_attr(feature = "cargo-clippy", allow(needless_range_loop))]
    fn ensure_sorted(&mut self) {
        if !self.is_sorted {
            // counting sort
            let mut root_occurences = vec![0; self.elements.len()];

            for idx in 0..self.elements.len() {
                root_occurences[self.find_root(idx)] += 1;
            }

            let mut root_start_index = root_occurences;

            let mut current_start_index = 0;

            for root in 0..self.elements.len() {
                //                                           still occurence count
                let next_start_index = current_start_index + root_start_index[root];
                // now start index
                root_start_index[root] = current_start_index;
                current_start_index = next_start_index;
            }

            let mut new_elements: Vec<T> = Vec::with_capacity(self.elements.len());
            let mut new_ranks: Vec<usize> = Vec::with_capacity(self.elements.len());
            let mut old_to_new_idx_map = vec![0; self.elements.len()];
            let mut new_to_old_idx_map = vec![0; self.elements.len()];

            for idx in 0..self.elements.len() {
                let root = self.find_root(idx);
                let new_idx = root_start_index[root];
                root_start_index[root] += 1;
                old_to_new_idx_map[idx] = new_idx;
                new_to_old_idx_map[new_idx] = idx;

                unsafe {
                    ::std::ptr::copy_nonoverlapping(
                        &self.elements[idx],
                        new_elements.as_mut_ptr().offset(new_idx as isize),
                        1,
                    );
                    ::std::ptr::copy_nonoverlapping(
                        &self.ranks[idx],
                        new_ranks.as_mut_ptr().offset(new_idx as isize),
                        1,
                    );
                }
            }

            unsafe {
                new_elements.set_len(self.elements.len());
                new_ranks.set_len(self.elements.len());
                // prevents items to be dropped, since they live on in new_elements
                self.elements.set_len(0);
            }

            self.elements = new_elements;
            self.ranks = new_ranks;
            self.parent_indices = new_to_old_idx_map
                .iter()
                .map(|&old_idx| old_to_new_idx_map[self.parent_indices[old_idx]])
                .collect();

            self.is_sorted = true;
        }
    }

    pub fn sets(&mut self) -> SetsIterator<T> {
        self.ensure_sorted();
        SetsIterator {
            elements: &self.elements,
            input_iter: self.parent_indices.iter().enumerate().peekable(),
        }
    }
}

pub struct SetsIterator<'a, T: 'a> {
    elements: &'a Vec<T>,
    input_iter: ::std::iter::Peekable<::std::iter::Enumerate<::std::slice::Iter<'a, usize>>>,
}

impl<'a, T: 'a> Iterator for SetsIterator<'a, T> {
    type Item = &'a [T];
    fn next(&mut self) -> Option<&'a [T]> {
        if let Some((set_start_idx, root)) = self.input_iter.next() {
            let mut set_end_idx = set_start_idx + 1;

            while self
                .input_iter
                .peek()
                .map(|&(_, next_root)| next_root == root)
                .unwrap_or(false)
            {
                self.input_iter.next();
                set_end_idx += 1;
            }

            Some(&self.elements[set_start_idx..set_end_idx])
        } else {
            None
        }
    }
}

#[test]
fn test_disjoint_sets() {
    let mut numbers = DisjointSets::from_individuals(vec![112, 44, 32, 66, 52, 74, 176]);
    numbers.union_all_with(|a, b| a % 10 == b % 10);
    println!("{:?}, {:?}", numbers.elements, numbers.parent_indices);
    numbers.ensure_sorted();
    println!("{:?}, {:?}", numbers.elements, numbers.parent_indices);
    let sets = numbers.sets();
    let set1 = [112, 32, 52];
    let set2 = [44, 74];
    let set3 = [66, 176];
    assert!(sets.collect::<Vec<_>>() == vec![&set1[..], &set2[..], &set3[..]]);
}
