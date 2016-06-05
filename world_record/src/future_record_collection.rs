use record_collection::RecordCollection;
use {ID, Record, FutureState};
use std::path::PathBuf;
use std::ops::Deref;

pub struct FutureRecordCollection<T> {
    collection: RecordCollection<T>,
    to_be_removed: Vec<ID<T>>,
    to_be_added: Vec<Record<T>>
}

impl<T> FutureRecordCollection<T> {
    pub fn new(path: PathBuf) -> FutureRecordCollection<T> {
        return FutureRecordCollection {
            collection: RecordCollection::<T>::new(path),
            to_be_removed: Vec::new(),
            to_be_added: Vec::new()
        }
    }
    
    pub fn add_soon(&mut self, mut record: Record<T>) -> &mut Record<T> {
        record.id = self.collection.reserve();
        self.to_be_added.push(record);
        return self.to_be_added.last_mut().unwrap();
    }
    
    pub fn remove_soon(&mut self, id: ID<T>) {
        self.to_be_removed.push(id);
    }
    
    pub fn overwrite_with(&mut self, other: &Self) {
        self.collection.overwrite_with(&other.collection);
    }
}

impl<T> FutureState for FutureRecordCollection<T> {
    fn materialize(&mut self) {
        for id_to_be_removed in self.to_be_removed.drain(..) {
            self.collection.remove(id_to_be_removed);
        }
        
        for record in self.to_be_added.drain(..) {
            self.collection.add(record);
        }
    }
}

impl<T> Deref for FutureRecordCollection<T> {
    type Target = RecordCollection<T>;
    
    fn deref (&self) -> &RecordCollection<T> {
        return &self.collection;
    }
}