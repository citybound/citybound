use std::collections::HashMap;
use std::intrinsics::{type_id, type_name};

pub struct TypeRegistry {
    next_short_id: usize,
    long_to_short_ids: HashMap<u64, usize>,
    short_ids_to_names: HashMap<usize, String>
}

impl TypeRegistry {
    pub fn new() -> TypeRegistry {
        TypeRegistry{
            next_short_id: 0,
            long_to_short_ids: HashMap::new(),
            short_ids_to_names: HashMap::new()
        }
    }

    pub fn register_new<T: 'static>(&mut self) -> usize {
        let short_id = self.next_short_id;
        let long_id = unsafe{type_id::<T>()};
        assert!(self.long_to_short_ids.get(&long_id).is_none());
        self.long_to_short_ids.insert(long_id, short_id);
        self.short_ids_to_names.insert(short_id, unsafe{type_name::<T>()}.into());
        self.next_short_id += 1;
        short_id
    }

    pub fn get_or_register<T: 'static>(&mut self) -> usize {
        let long_id = unsafe{type_id::<T>()};
        if let Some(existing_short_id) = self.long_to_short_ids.get(&long_id) {
            return *existing_short_id;
        }

        let short_id = self.next_short_id;
        self.long_to_short_ids.insert(long_id, short_id);
        self.short_ids_to_names.insert(short_id, unsafe{type_name::<T>()}.into());
        self.next_short_id += 1;
        short_id
    }

    pub fn get<T: 'static>(&self) -> usize {
        *self.long_to_short_ids.get(&unsafe{type_id::<T>()}).expect((format!("{:?} not known.", &unsafe{type_name::<T>()})).as_str())
    }

    pub fn get_name(&self, short_id: usize) -> &String {
        &self.short_ids_to_names[&short_id]
    }
}