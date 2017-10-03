use descartes::{N, BoundingBox};
use fnv::{FnvHashMap, FnvHashSet};
use roaring::RoaringBitmap;

pub struct GridAccelerator {
    cells: FnvHashMap<(isize, isize), RoaringBitmap>,
    cells_of: FnvHashMap<usize, Vec<(isize, isize)>>,
    cell_size: N,
    colocated_pairs: Vec<(usize, RoaringBitmap)>,
}

impl GridAccelerator {
    pub fn new(cell_size: N) -> Self {
        GridAccelerator {
            cells: FnvHashMap::default(),
            cells_of: FnvHashMap::default(),
            cell_size: cell_size,
            colocated_pairs: Vec::default(),
        }
    }

    #[inline(never)]
    pub fn add<I: Iterator<Item = BoundingBox>>(&mut self, new_ref: usize, bboxes: I) {
        assert!(self.cells_of.get(&new_ref).is_none(), "already added");
        let mut new_pair_partners = RoaringBitmap::new();
        self.cells_of.insert(new_ref, Vec::new());
        for bbox in bboxes {
            let x_start = (bbox.min.x / self.cell_size).floor() as isize;
            let x_end = (bbox.max.x / self.cell_size).floor() as isize + 1;
            let y_start = (bbox.min.y / self.cell_size).floor() as isize;
            let y_end = (bbox.max.y / self.cell_size).floor() as isize + 1;
            for cell_x in x_start..x_end {
                for cell_y in y_start..y_end {
                    let cell = self.cells.entry((cell_x, cell_y)).or_insert_with(
                        RoaringBitmap::new,
                    );
                    new_pair_partners.union_with(cell);
                    cell.insert(new_ref as u32);
                    self.cells_of.get_mut(&new_ref).unwrap().push(
                        (cell_x, cell_y),
                    );
                }
            }
        }
        self.colocated_pairs.push((new_ref, new_pair_partners));
    }

    pub fn colocated_for(&self, query_ref: usize) -> Vec<usize> {
        self.cells_of
            .get(&query_ref)
            .into_iter()
            .flat_map(|coordinates_set| {
                coordinates_set.iter().flat_map(|coordinates| {
                    self.cells[coordinates].iter().map(
                        |ref_u64| ref_u64 as usize,
                    )
                })
            })
            .collect::<FnvHashSet<_>>()
            .into_iter()
            .filter(|found_ref| found_ref != &query_ref)
            .collect::<Vec<_>>()
    }


    #[inline(never)]
    // TODO: wrap the return in something nicer
    pub fn colocated_pairs(&self) -> &Vec<(usize, RoaringBitmap)> {
        &self.colocated_pairs
    }
}
