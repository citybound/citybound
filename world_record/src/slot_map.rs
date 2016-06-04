use growable_vec::GrowableVec;
use std::path::PathBuf;
use std::ops::{Index};
use std::marker::PhantomData;
use {ID, IDRepr};

pub struct SlotMap<T> {
    ids_to_slots: GrowableVec<usize>,
    free_ids: GrowableVec<ID<T>>,
}

impl<T> SlotMap<T> {
    pub fn new(path: PathBuf) -> SlotMap<T> {
        return SlotMap {
            ids_to_slots: GrowableVec::new(path.join("ids_to_slots")),
            free_ids: GrowableVec::new(path.join("free_ids")),
        };
    }

    pub fn reserve(&mut self) -> ID<T> {
        match self.free_ids.pop() {
            Some(id) => id,
            None => {
                self.ids_to_slots.push(usize::max_value());
                return ID{
                    id: (self.ids_to_slots.len() - 1) as IDRepr,
                    marker: PhantomData::<T>
                };
            }
        }
    }

    pub fn assoc(&mut self, id: ID<T>, slot: usize) {
        self.ids_to_slots[id.id as usize] = slot;
    }

    pub fn release(&mut self, id: ID<T>) {
        self.ids_to_slots[id.id as usize] = usize::max_value();
        self.free_ids.push(id);
    }
}

impl<T> Index<ID<T>> for SlotMap<T> {
    type Output = usize;

    fn index(&self, id: ID<T>) -> &Self::Output {
        return &self.ids_to_slots[id.id as usize];
    }
}