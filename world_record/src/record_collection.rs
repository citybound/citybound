use growable_vec::GrowableVec;
use {ID, Record};
use slot_map::SlotMap;
use std::ops::{Index, IndexMut};
use std::path::PathBuf;

pub struct RecordCollection<T> {
    records: GrowableVec<Record<T>>,
    slot_map: SlotMap<T>,
}

impl<T> RecordCollection<T> {
    pub fn new(path: PathBuf) -> RecordCollection<T> {
        return RecordCollection {
            records: GrowableVec::new(path.join("records")),
            slot_map: SlotMap::<T>::new(path.join("slot_map")),
        };
    }

    pub fn add(&mut self, record: Record<T>) {
        let slot = self.records.len();
        let id = record.id;
        self.records.push(record);
        self.slot_map.assoc(id, slot);
    }

    pub fn remove(&mut self, id: ID<T>) {
        let slot = self.slot_map[id];
        self.records.swap_remove(slot);
        let ref new_record_at_slot = self.records[slot];
        self.slot_map.assoc(new_record_at_slot.id, slot);
        self.slot_map.release(id);
    }

    pub fn reserve(&mut self) -> ID<T> {
        return self.slot_map.reserve();
    }
}

impl<T> Index<ID<T>> for RecordCollection<T> {
    type Output = Record<T>;

    fn index(&self, id: ID<T>) -> &Self::Output {
        return &self.records[self.slot_map[id]];
    }
}

impl<T> IndexMut<ID<T>> for RecordCollection<T> {
    fn index_mut(&mut self, id: ID<T>) -> &mut Self::Output {
        return &mut self.records[self.slot_map[id]];
    }
}
