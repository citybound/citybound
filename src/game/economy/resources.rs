use std::collections::HashMap;
use core::read_md_tables::read;

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct ResourceId(u16);

impl ::std::fmt::Debug for ResourceId {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        write!(f,
               "r({})",
               unsafe { &(*REGISTRY).id_to_info.get(self).unwrap().0 })
    }
}

#[derive(Debug)]
struct ResourceDescription(String, String);

#[derive(Default)]
pub struct ResourceRegistry {
    next_id: ResourceId,
    name_to_id: HashMap<String, ResourceId>,
    id_to_info: HashMap<ResourceId, ResourceDescription>,
}

impl ResourceRegistry {
    fn add(&mut self, resource: &str, description: &str) {
        self.name_to_id.insert(resource.to_owned(), self.next_id);
        self.id_to_info.insert(self.next_id,
                               ResourceDescription(resource.to_owned(), description.to_owned()));
        self.next_id = match self.next_id {
            ResourceId(id) => ResourceId(id + 1),
        };
    }

    fn id(&self, resource: &str) -> ResourceId {
        *self.name_to_id
            .get(resource)
            .expect(format!("Resource {} doesn't exist. Loaded resources: {:?}",
                            resource,
                            self.name_to_id)
                .as_str())
    }
}

pub static mut REGISTRY: *mut ResourceRegistry = 0 as *mut ResourceRegistry;

pub fn r(resource: &str) -> ResourceId {
    unsafe { (*REGISTRY).id(resource) }
}

pub fn setup() {
    let mut resources = Box::new(ResourceRegistry::default());
    let tables = read(&"./simulation_parameters/resources").unwrap();

    for table in &tables {
        let c = &table.columns;
        for (resource, info) in c["resource"].iter().zip(&c["description"]) {
            resources.add(resource, info);
        }
    }

    unsafe {
        REGISTRY = Box::into_raw(resources);
    }
}

use compact::CVec;

pub type ResourceValue = f32;

#[derive(Compact, Clone, Default, Debug)]
pub struct Bag {
    entries: CVec<(ResourceId, ResourceValue)>,
}

pub struct BagEntry<'a> {
    entry: &'a mut ResourceValue,
}

impl Bag {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn get(&self, key: ResourceId) -> Option<&ResourceValue> {
        self.entries.binary_search_by_key(&key, |&(k, _v)| k).ok().map(|i| &self.entries[i].1)
    }

    pub fn mut_entry_or(&mut self, key: ResourceId, default: ResourceValue) -> &mut ResourceValue {
        match self.entries.binary_search_by_key(&key, |&(k, _v)| k) {
            Ok(index) => &mut self.entries[index].1,
            Err(index) => {
                self.entries.insert(index, (key, default));
                &mut self.entries[index].1
            }
        }
    }

    pub fn mut_entry_or_id(&mut self, key: ResourceId, default: u32) -> &mut u32 {
        match self.entries.binary_search_by_key(&key, |&(k, _v)| k) {
            Ok(index) => unsafe {
                ::std::mem::transmute::<&mut ResourceValue, &mut u32>(&mut self.entries[index].1)
            },
            Err(index) => {
                self.entries.insert(index,
                                    (key,
                                     unsafe {
                                         ::std::mem::transmute::<u32, ResourceValue>(default)
                                     }));
                unsafe {
                    ::std::mem::transmute::<&mut ResourceValue, &mut u32>(&mut self.entries[index]
                        .1)
                }
            }
        }
    }
}