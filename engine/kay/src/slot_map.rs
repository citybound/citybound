use super::chunked::{Chunker, ChunkedVec};

#[derive(Clone, Copy)]
pub struct SlotIndices {
    bin: u8,
    slot: u32,
}

impl SlotIndices {
    pub fn new(bin: usize, slot: usize) -> SlotIndices {
        SlotIndices { bin: bin as u8, slot: slot as u32 }
    }

    pub fn invalid() -> SlotIndices {
        SlotIndices {
            bin: u8::max_value(),
            slot: u32::max_value(),
        }
    }

    pub fn bin(&self) -> usize {
        self.bin as usize
    }

    pub fn slot(&self) -> usize {
        self.slot as usize
    }
}

pub struct SlotMap {
    entries: ChunkedVec<SlotIndices>,
    last_known_version: ChunkedVec<u8>,
    free_ids_with_versions: ChunkedVec<(usize, usize)>,
}

impl SlotMap {
    pub fn new(chunker: Box<Chunker>) -> Self {
        SlotMap {
            entries: ChunkedVec::new(chunker.child("_entries")),
            last_known_version: ChunkedVec::new(chunker.child("_last_known_version")),
            free_ids_with_versions: ChunkedVec::new(chunker.child("_free_ids_with_versions")),
        }
    }

    pub fn allocate_id(&mut self) -> (usize, usize) {
        match self.free_ids_with_versions.pop() {
            None => {
                self.entries.push(SlotIndices::invalid());
                self.last_known_version.push(0);
                (self.entries.len() - 1, 0)
            }
            Some((id, version)) => (id, version),
        }
    }

    pub fn associate(&mut self, id: usize, new_entry: SlotIndices) {
        let entry = self.entries.at_mut(id);
        entry.clone_from(&new_entry);
    }

    pub fn indices_of(&self, id: usize, version: u8) -> Option<SlotIndices> {
        if *self.last_known_version.at(id) == version {
            Some(self.indices_of_no_version_check(id))
        } else {
            None
        }
    }

    pub fn indices_of_no_version_check(&self, id: usize) -> SlotIndices {
        *self.entries.at(id)
    }

    pub fn free(&mut self, id: usize, version: usize) {
        *self.last_known_version.at_mut(id) = (version + 1) as u8;
        self.free_ids_with_versions.push((id, version + 1));
    }
}
