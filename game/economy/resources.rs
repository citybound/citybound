use std::collections::HashMap;
use core::read_md_tables;
use itertools::multizip;

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct ResourceId(u16);

pub const MAX_N_RESOURCE_TYPES: usize = 1000;

impl ResourceId {
    pub fn as_index(&self) -> usize {
        self.0 as usize
    }
}

impl ::std::fmt::Debug for ResourceId {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        write!(f, "r({})", unsafe { &(*REGISTRY).id_to_info[self].0 })
    }
}

#[derive(Clone, Debug)]
pub struct ResourceDescription(pub String, pub String);

#[derive(Default, Copy, Clone)]
pub struct ResourceProperties {
    pub ownership_shared: bool,
    pub supplier_shared: bool,
}

pub struct ResourceRegistry {
    next_id: ResourceId,
    name_to_id: HashMap<String, ResourceId>,
    id_to_info: HashMap<ResourceId, ResourceDescription>,
    properties: [ResourceProperties; MAX_N_RESOURCE_TYPES],
}

impl Default for ResourceRegistry {
    fn default() -> Self {
        ResourceRegistry {
            next_id: ResourceId::default(),
            name_to_id: HashMap::default(),
            id_to_info: HashMap::default(),
            properties: [ResourceProperties::default(); MAX_N_RESOURCE_TYPES],
        }
    }
}

impl ResourceRegistry {
    fn add(
        &mut self,
        resource: &str,
        description: &str,
        ownership_shared: bool,
        supplier_shared: bool,
    ) {
        let id = self.next_id;
        self.name_to_id.insert(resource.to_owned(), id);
        self.id_to_info.insert(
            id,
            ResourceDescription(resource.to_owned(), description.to_owned()),
        );
        self.properties[id.as_index()] = ResourceProperties { ownership_shared, supplier_shared };
        self.next_id = match self.next_id {
            ResourceId(id) => ResourceId(id + 1),
        };
    }

    fn id(&self, resource: &str) -> ResourceId {
        if let Some(&resource_id) = self.name_to_id.get(resource) {
            resource_id
        } else {
            panic!(
                "Resource {} doesn't exist. Loaded resources: {:?}",
                resource,
                self.name_to_id
            )
        }
    }
}

pub static mut REGISTRY: *mut ResourceRegistry = 0 as *mut ResourceRegistry;

pub fn r_id(resource: &str) -> ResourceId {
    unsafe { (*REGISTRY).id(resource) }
}

pub fn r_info(resource_id: ResourceId) -> ResourceDescription {
    unsafe { (*REGISTRY).id_to_info[&resource_id].clone() }
}

pub fn r_properties(resource_id: ResourceId) -> ResourceProperties {
    unsafe { (*REGISTRY).properties[resource_id.as_index()] }
}

pub fn all_resource_ids() -> Vec<ResourceId> {
    unsafe { (*REGISTRY).id_to_info.keys().cloned().collect() }
}

pub fn setup() {
    let mut resources = Box::new(ResourceRegistry::default());
    let tables = read_md_tables::read_str(include_str!("parameters/resources/default.data.md"))
        .unwrap();

    for table in &tables {
        let c = &table.columns;
        for (resource, own, sup, info) in
            multizip((
                &c["resource"],
                &c["ownership"],
                &c["supplier"],
                &c["description"],
            ))
        {
            resources.add(resource, info, own == "shared", sup == "shared");
        }
    }

    unsafe {
        REGISTRY = Box::into_raw(resources);
    }
}

use compact::{CVec, Compact};

pub type ResourceAmount = f32;

#[derive(Compact, Clone, Debug)]
pub struct Entry<AssociatedValue: Compact>(pub ResourceId, pub AssociatedValue);

#[derive(Compact, Clone, Debug)]
pub struct ResourceMap<AssociatedValue: Compact> {
    entries: CVec<Entry<AssociatedValue>>,
}

impl<AssociatedValue: Compact> Default for ResourceMap<AssociatedValue> {
    fn default() -> Self {
        ResourceMap { entries: CVec::new() }
    }
}

impl<AssociatedValue: Compact> ResourceMap<AssociatedValue> {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn get(&self, key: ResourceId) -> Option<&AssociatedValue> {
        self.entries
            .binary_search_by_key(&key, |&Entry(ref k, ref _v)| *k)
            .ok()
            .map(|i| &self.entries[i].1)
    }

    pub fn mut_entry_or(
        &mut self,
        key: ResourceId,
        default: AssociatedValue,
    ) -> &mut AssociatedValue {
        match self.entries.binary_search_by_key(
            &key,
            |&Entry(ref k, ref _v)| *k,
        ) {
            Ok(index) => &mut self.entries[index].1,
            Err(index) => {
                self.entries.insert(index, Entry(key, default));
                &mut self.entries[index].1
            }
        }
    }

    pub fn insert(&mut self, key: ResourceId, value: AssociatedValue) -> Option<AssociatedValue> {
        match self.entries.binary_search_by_key(
            &key,
            |&Entry(ref k, ref _v)| *k,
        ) {
            Ok(index) => {
                let old = ::std::mem::replace(&mut self.entries[index].1, value);
                Some(old)
            }
            Err(index) => {
                self.entries.insert(index, Entry(key, value));
                None
            }
        }
    }

    pub fn remove(&mut self, key: ResourceId) -> Option<AssociatedValue> {
        match self.entries.binary_search_by_key(
            &key,
            |&Entry(ref k, ref _v)| *k,
        ) {
            Ok(index) => Some(self.entries.remove(index).1),
            Err(_) => None,
        }
    }
}

impl<AssociatedValue: Compact> ::std::ops::Deref for ResourceMap<AssociatedValue> {
    type Target = CVec<Entry<AssociatedValue>>;

    fn deref(&self) -> &Self::Target {
        &self.entries
    }
}

impl<AssociatedValue: Compact> ::std::iter::FromIterator<(ResourceId, AssociatedValue)>
    for ResourceMap<AssociatedValue> {
    fn from_iter<T: IntoIterator<Item = (ResourceId, AssociatedValue)>>(iter: T) -> Self {
        let mut map = Self::new();
        for (resource, value) in iter {
            map.insert(resource, value);
        }
        map
    }
}

pub type Inventory = ResourceMap<ResourceAmount>;

impl Inventory {
    pub fn give_to(&self, target: &mut Inventory) {
        for &Entry(resource, delta) in self.iter() {
            *(target.mut_entry_or(resource, 0.0)) += delta;
        }
    }

    pub fn take_from(&self, target: &mut Inventory) {
        for &Entry(resource, delta) in self.iter() {
            *(target.mut_entry_or(resource, 0.0)) -= delta;
        }
    }

    pub fn give_to_shared_private(&self, shared: &mut Inventory, private: &mut Inventory) {
        for &Entry(resource, delta) in self.iter() {
            if r_properties(resource).ownership_shared {
                *(shared.mut_entry_or(resource, 0.0)) += delta;
            } else {
                *(private.mut_entry_or(resource, 0.0)) += delta;
            }
        }
    }

    pub fn take_from_shared_private(&self, shared: &mut Inventory, private: &mut Inventory) {
        for &Entry(resource, delta) in self.iter() {
            if r_properties(resource).ownership_shared {
                *(shared.mut_entry_or(resource, 0.0)) -= delta;
            } else {
                *(private.mut_entry_or(resource, 0.0)) -= delta;
            }
        }
    }
}