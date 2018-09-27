#[derive(Copy, Clone, Debug, Hash, PartialOrd, Ord, PartialEq, Eq, Serialize, Deserialize)]
pub enum Resource {
    Awakeness,
    Satiety,
    //Entertainment,
    //Services,
    Money,
    Groceries,
    Produce,
    Grain,
    Flour,
    BakedGoods,
    Meat,
    DairyGoods,
    /* Wood, */
    /*Furniture,
     *TextileGoods,
     *Clothes,
     *Devices, */
}

use self::Resource::*;

impl ::std::fmt::Display for Resource {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        ::std::fmt::Debug::fmt(&self, f)
    }
}

impl Resource {
    pub fn description(&self) -> &'static str {
        match *self {
            Awakeness => "How much energy a person has.",
            Satiety => "How little hungry a person is.",
            // Entertainment => "How entertained a person is.",
            // Services => "How many services a person or business needs.",
            Money => "Money.",
            Groceries => "Mixed food for daily consumption.",
            Produce => "Agricultural fruits & vegeteables produce",
            Grain => "Agricultural grain produce",
            Flour => "Processed Grains",
            BakedGoods => "Baked Goods",
            Meat => "Meat",
            DairyGoods => "Dairy Goods",
            /* Wood => "Wood",
             * Furniture => "Furniture",
             * TextileGoods => "Textile Goods",
             * Clothes => "Clothes",
             * Devices => "Devices", */
        }
    }
}

use compact::{CVec, Compact};

pub type ResourceAmount = f32;

#[derive(Compact, Clone, Debug, Serialize, Deserialize)]
pub struct Entry<AssociatedValue: Compact>(pub Resource, pub AssociatedValue);

#[derive(Compact, Clone, Debug, Serialize, Deserialize)]
pub struct ResourceMap<AssociatedValue: Compact> {
    entries: CVec<Entry<AssociatedValue>>,
}

impl<AssociatedValue: Compact> Default for ResourceMap<AssociatedValue> {
    fn default() -> Self {
        ResourceMap {
            entries: CVec::new(),
        }
    }
}

impl<AssociatedValue: Compact> ResourceMap<AssociatedValue> {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn get(&self, key: Resource) -> Option<&AssociatedValue> {
        self.entries
            .binary_search_by_key(&key, |&Entry(ref k, ref _v)| *k)
            .ok()
            .map(|i| &self.entries[i].1)
    }

    pub fn mut_entry_or(
        &mut self,
        key: Resource,
        default: AssociatedValue,
    ) -> &mut AssociatedValue {
        match self
            .entries
            .binary_search_by_key(&key, |&Entry(ref k, ref _v)| *k)
        {
            Ok(index) => &mut self.entries[index].1,
            Err(index) => {
                self.entries.insert(index, Entry(key, default));
                &mut self.entries[index].1
            }
        }
    }

    pub fn insert(&mut self, key: Resource, value: AssociatedValue) -> Option<AssociatedValue> {
        match self
            .entries
            .binary_search_by_key(&key, |&Entry(ref k, ref _v)| *k)
        {
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

    pub fn remove(&mut self, key: Resource) -> Option<AssociatedValue> {
        match self
            .entries
            .binary_search_by_key(&key, |&Entry(ref k, ref _v)| *k)
        {
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

impl<AssociatedValue: Compact> ::std::iter::FromIterator<(Resource, AssociatedValue)>
    for ResourceMap<AssociatedValue>
{
    fn from_iter<T: IntoIterator<Item = (Resource, AssociatedValue)>>(iter: T) -> Self {
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

    pub fn give_to_shared_private<F: Fn(Resource) -> bool>(
        &self,
        shared: &mut Inventory,
        private: &mut Inventory,
        is_shared: F,
    ) {
        for &Entry(resource, delta) in self.iter() {
            if is_shared(resource) {
                *(shared.mut_entry_or(resource, 0.0)) += delta;
            } else {
                *(private.mut_entry_or(resource, 0.0)) += delta;
            }
        }
    }

    pub fn take_from_shared_private<F: Fn(Resource) -> bool>(
        &self,
        shared: &mut Inventory,
        private: &mut Inventory,
        is_shared: F,
    ) {
        for &Entry(resource, delta) in self.iter() {
            if is_shared(resource) {
                *(shared.mut_entry_or(resource, 0.0)) -= delta;
            } else {
                *(private.mut_entry_or(resource, 0.0)) -= delta;
            }
        }
    }
}
