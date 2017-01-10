use super::chunked::{Chunker, ChunkedVec};

/// The index into a `MultiSized<SizedChunkedArena>`
#[derive(Clone, Copy)]
pub struct SlotIndices {
    /// Index of sized collection that contains the item
    collection: u8,
    /// The slot within those chunks holding the data
    slot: u32
}

impl SlotIndices {
    /// Create a new indices
    pub fn new(collection: usize, slot: usize) -> SlotIndices {
        SlotIndices {
            collection: collection as u8,
            slot: slot as u32
        }
    }

    /// Create a new, invalid indices
    pub fn invalid() -> SlotIndices {
        SlotIndices {
            collection: u8::max_value(),
            slot: u32::max_value()
        }
    }

    pub fn collection(&self) -> usize {
        self.collection as usize
    }

    pub fn slot(&self) -> usize {
        self.slot as usize
    }
}

/// Allows the lockup of the indices by an actor's ID
pub struct SlotMap {
    entries: ChunkedVec<SlotIndices>,
    free_ids_with_versions: ChunkedVec<(usize, usize)>
}

use random::Source;

impl SlotMap {
    /// Create a new `SlotMap`
    pub fn new(chunker: Box<Chunker>) -> Self {
        SlotMap {
            entries: ChunkedVec::new(chunker.child("_entries")),
            free_ids_with_versions: ChunkedVec::new(chunker.child("_free_ids_with_versions"))
        }
    }

    /// Allocate a ID either by allocating a new entry or using an existing, free one
    pub fn allocate_id(&mut self) -> (usize, usize) {
        match self.free_ids_with_versions.pop() {
            None => {
                self.entries.push(SlotIndices::invalid());
                (self.entries.len() - 1, 0)
            },
            Some(free_id) => free_id
        }
    }

    /// Set the indices at the ID
    pub fn associate(&mut self, id: usize, new_entry: SlotIndices) {
        let entry = self.entries.at_mut(id);
        entry.clone_from(&new_entry);
    }

    /// Lookup the indices at the ID
    pub fn indices_of(&self, id: usize) -> &SlotIndices {
        self.entries.at(id)
    }

    /// Mark an ID as free for reuse
    pub fn free(&mut self, id: usize, version: usize) {
        self.free_ids_with_versions.push((id, version + 1));
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Get an ID which is currently in use for messages with random recipients
    pub fn random_used(&self) -> usize {
        loop {
            let random_id = ::random::default().read::<usize>() % self.entries.len();
            let mut is_free = false;
            for i in 0..self.free_ids_with_versions.len() {
                if self.free_ids_with_versions.at(i).0 == random_id {
                    is_free = true
                }
            }
            if !is_free {return random_id}
        }
    }
}