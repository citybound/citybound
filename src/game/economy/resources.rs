use std::collections::HashMap;
use core::read_md_tables::read;

#[derive(Copy, Clone, PartialEq, Eq, Hash, Default)]
pub struct ResourceId(u16);

impl ::std::fmt::Debug for ResourceId {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        write!(f,
               "r({})",
               unsafe { &(*REGISTRY).id_to_info.get(self).unwrap().0 })
    }
}



#[derive(Debug)]
struct ResourceDescription(String, String, String);

#[derive(Default)]
pub struct ResourceRegistry {
    next_id: ResourceId,
    name_to_id: HashMap<String, ResourceId>,
    id_to_info: HashMap<ResourceId, ResourceDescription>,
}

impl ResourceRegistry {
    fn add(&mut self, resource: &str, unit: &str, description: &str) {
        self.name_to_id.insert(resource.to_owned(), self.next_id);
        self.id_to_info.insert(self.next_id,
                               ResourceDescription(resource.to_owned(),
                                                   unit.to_owned(),
                                                   description.to_owned()));
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
    let c = &read(&"./src/game/economy/instances/resources.data.md").unwrap()[0].columns;

    for (resource, (unit, info)) in
        c["resource"].iter().zip(c["unit"].iter().zip(&c["description"])) {
        resources.add(resource, unit, info);
    }

    unsafe {
        REGISTRY = Box::into_raw(resources);
    }
}