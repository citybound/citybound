use std::collections::HashMap;
use std::intrinsics::{type_id, type_name};

/// Provides lookups between:
/// 1. Rust `TypeId`s (long IDs) and sequential, internal type IDs (short IDs)
/// 2. sequential, internal type IDs (short IDs) to the human readable name of the type
pub struct TypeRegistry {
    next_short_id: usize,
    long_to_short_ids: HashMap<u64, usize>,
    short_ids_to_names: HashMap<usize, String>
}

impl TypeRegistry {
    /// Create a new, empty type registry
    pub fn new() -> TypeRegistry {
        TypeRegistry{
            next_short_id: 0,
            long_to_short_ids: HashMap::new(),
            short_ids_to_names: HashMap::new()
        }
    }

    /// register a new type in the registry
    pub fn register_new<T: 'static>(&mut self) -> usize {
        let short_id = self.next_short_id;
        let long_id = unsafe{type_id::<T>()};
        assert!(self.long_to_short_ids.get(&long_id).is_none());
        self.long_to_short_ids.insert(long_id, short_id);
        self.short_ids_to_names.insert(short_id, unsafe{type_name::<T>()}.into());
        self.next_short_id += 1;
        short_id
    }

    /// Get the sequential, internal type IDs (short IDs) from a type
    pub fn get<T: 'static>(&self) -> usize {
        *self.long_to_short_ids.get(&unsafe{type_id::<T>()}).expect((format!("{:?} not known.", &unsafe{type_name::<T>()})).as_str())
    }

    /// Get the human readable type name from the sequential, internal type IDs (short IDs)
    pub fn get_name(&self, short_id: usize) -> &String {
        &self.short_ids_to_names[&short_id]
    }
}