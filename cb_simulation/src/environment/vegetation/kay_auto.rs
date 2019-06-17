//! This is all auto-generated. Do not touch.
#![rustfmt::skip]
#[allow(unused_imports)]
use kay::{ActorSystem, TypedID, RawID, Fate, Actor, TraitIDFrom, ActorOrActorTrait};
#[allow(unused_imports)]
use super::*;



impl Actor for Plant {
    type ID = PlantID;

    fn id(&self) -> Self::ID {
        self.id
    }
    unsafe fn set_id(&mut self, id: RawID) {
        self.id = Self::ID::from_raw(id);
    }
}

#[derive(Serialize, Deserialize)] #[serde(transparent)]
pub struct PlantID {
    _raw_id: RawID
}

impl Copy for PlantID {}
impl Clone for PlantID { fn clone(&self) -> Self { *self } }
impl ::std::fmt::Debug for PlantID {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        write!(f, "PlantID({:?})", self._raw_id)
    }
}
impl ::std::hash::Hash for PlantID {
    fn hash<H: ::std::hash::Hasher>(&self, state: &mut H) {
        self._raw_id.hash(state);
    }
}
impl PartialEq for PlantID {
    fn eq(&self, other: &PlantID) -> bool {
        self._raw_id == other._raw_id
    }
}
impl Eq for PlantID {}

impl TypedID for PlantID {
    type Target = Plant;

    fn from_raw(id: RawID) -> Self {
        PlantID { _raw_id: id }
    }

    fn as_raw(&self) -> RawID {
        self._raw_id
    }
}

impl PlantID {
    pub fn spawn(proto: PlantPrototype, world: &mut World) -> Self {
        let id = PlantID::from_raw(world.allocate_instance_id::<Plant>());
        let swarm = world.local_broadcast::<Plant>();
        world.send(swarm, MSG_Plant_spawn(id, proto));
        id
    }
}

#[derive(Compact, Clone)] #[allow(non_camel_case_types)]
struct MSG_Plant_spawn(pub PlantID, pub PlantPrototype);

impl Into<ConstructableID<CBPrototypeKind>> for PlantID {
    fn into(self) -> ConstructableID<CBPrototypeKind> {
        ConstructableID::from_raw(self.as_raw())
    }
}

#[allow(unused_variables)]
#[allow(unused_mut)]
pub fn auto_setup(system: &mut ActorSystem) {
    
    ConstructableID::<CBPrototypeKind>::register_implementor::<Plant>(system);
    system.add_spawner::<Plant, _, _>(
        |&MSG_Plant_spawn(id, proto), world| {
            Plant::spawn(id, proto, world)
        }, false
    );
}