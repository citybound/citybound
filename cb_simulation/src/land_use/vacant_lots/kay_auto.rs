//! This is all auto-generated. Do not touch.
#![rustfmt::skip]
#[allow(unused_imports)]
use kay::{ActorSystem, TypedID, RawID, Fate, Actor, TraitIDFrom, ActorOrActorTrait};
#[allow(unused_imports)]
use super::*;



impl Actor for VacantLot {
    type ID = VacantLotID;

    fn id(&self) -> Self::ID {
        self.id
    }
    unsafe fn set_id(&mut self, id: RawID) {
        self.id = Self::ID::from_raw(id);
    }
}

#[derive(Serialize, Deserialize)] #[serde(transparent)]
pub struct VacantLotID {
    _raw_id: RawID
}

impl Copy for VacantLotID {}
impl Clone for VacantLotID { fn clone(&self) -> Self { *self } }
impl ::std::fmt::Debug for VacantLotID {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        write!(f, "VacantLotID({:?})", self._raw_id)
    }
}
impl ::std::hash::Hash for VacantLotID {
    fn hash<H: ::std::hash::Hasher>(&self, state: &mut H) {
        self._raw_id.hash(state);
    }
}
impl PartialEq for VacantLotID {
    fn eq(&self, other: &VacantLotID) -> bool {
        self._raw_id == other._raw_id
    }
}
impl Eq for VacantLotID {}

impl TypedID for VacantLotID {
    type Target = VacantLot;

    fn from_raw(id: RawID) -> Self {
        VacantLotID { _raw_id: id }
    }

    fn as_raw(&self) -> RawID {
        self._raw_id
    }
}

impl VacantLotID {
    pub fn spawn(lot: Lot, based_on: PrototypeID, world: &mut World) -> Self {
        let id = VacantLotID::from_raw(world.allocate_instance_id::<VacantLot>());
        let swarm = world.local_broadcast::<VacantLot>();
        world.send(swarm, MSG_VacantLot_spawn(id, lot, based_on));
        id
    }
    
    pub fn suggest_lot(self, building_style: BuildingStyle, requester: DevelopmentManagerID, world: &mut World) {
        world.send(self.as_raw(), MSG_VacantLot_suggest_lot(building_style, requester));
    }
}

#[derive(Compact, Clone)] #[allow(non_camel_case_types)]
struct MSG_VacantLot_spawn(pub VacantLotID, pub Lot, pub PrototypeID);
#[derive(Compact, Clone)] #[allow(non_camel_case_types)]
struct MSG_VacantLot_suggest_lot(pub BuildingStyle, pub DevelopmentManagerID);

impl Into<ConstructableID> for VacantLotID {
    fn into(self) -> ConstructableID {
        ConstructableID::from_raw(self.as_raw())
    }
}

#[allow(unused_variables)]
#[allow(unused_mut)]
pub fn auto_setup(system: &mut ActorSystem) {
    
    ConstructableID::register_implementor::<VacantLot>(system);
    system.add_spawner::<VacantLot, _, _>(
        |&MSG_VacantLot_spawn(id, ref lot, based_on), world| {
            VacantLot::spawn(id, lot, based_on, world)
        }, false
    );
    
    system.add_handler::<VacantLot, _, _>(
        |&MSG_VacantLot_suggest_lot(building_style, requester), instance, world| {
            instance.suggest_lot(building_style, requester, world); Fate::Live
        }, false
    );
}