//! This is all auto-generated. Do not touch.
#![rustfmt::skip]
#[allow(unused_imports)]
use kay::{ActorSystem, TypedID, RawID, Fate, Actor, TraitIDFrom, ActorOrActorTrait};
#[allow(unused_imports)]
use super::*;



impl Actor for Building {
    type ID = BuildingID;

    fn id(&self) -> Self::ID {
        self.id
    }
    unsafe fn set_id(&mut self, id: RawID) {
        self.id = Self::ID::from_raw(id);
    }
}

#[derive(Serialize, Deserialize)] #[serde(transparent)]
pub struct BuildingID {
    _raw_id: RawID
}

impl Copy for BuildingID {}
impl Clone for BuildingID { fn clone(&self) -> Self { *self } }
impl ::std::fmt::Debug for BuildingID {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        write!(f, "BuildingID({:?})", self._raw_id)
    }
}
impl ::std::hash::Hash for BuildingID {
    fn hash<H: ::std::hash::Hasher>(&self, state: &mut H) {
        self._raw_id.hash(state);
    }
}
impl PartialEq for BuildingID {
    fn eq(&self, other: &BuildingID) -> bool {
        self._raw_id == other._raw_id
    }
}
impl Eq for BuildingID {}

impl TypedID for BuildingID {
    type Target = Building;

    fn from_raw(id: RawID) -> Self {
        BuildingID { _raw_id: id }
    }

    fn as_raw(&self) -> RawID {
        self._raw_id
    }
}

impl BuildingID {
    pub fn spawn(style: BuildingStyle, lot: Lot, world: &mut World) -> Self {
        let id = BuildingID::from_raw(world.allocate_instance_id::<Building>());
        let swarm = world.local_broadcast::<Building>();
        world.send(swarm, MSG_Building_spawn(id, style, lot));
        id
    }
    
    pub fn try_offer_unit(self, required_unit_type: UnitType, requester: ImmigrationManagerID, world: &mut World) {
        world.send(self.as_raw(), MSG_Building_try_offer_unit(required_unit_type, requester));
    }
    
    pub fn add_household(self, household: HouseholdID, unit: UnitIdx, world: &mut World) {
        world.send(self.as_raw(), MSG_Building_add_household(household, unit));
    }
    
    pub fn remove_household(self, household: HouseholdID, world: &mut World) {
        world.send(self.as_raw(), MSG_Building_remove_household(household));
    }
    
    pub fn finally_destroy(self, world: &mut World) {
        world.send(self.as_raw(), MSG_Building_finally_destroy());
    }
    
    pub fn get_ui_info(self, requester: LandUseUIID, world: &mut World) {
        world.send(self.as_raw(), MSG_Building_get_ui_info(requester));
    }
    
    pub fn reconnect(self, new_location: PreciseLocation, new_connection_point: P2, world: &mut World) {
        world.send(self.as_raw(), MSG_Building_reconnect(new_location, new_connection_point));
    }
}

#[derive(Compact, Clone)] #[allow(non_camel_case_types)]
struct MSG_Building_spawn(pub BuildingID, pub BuildingStyle, pub Lot);
#[derive(Compact, Clone)] #[allow(non_camel_case_types)]
struct MSG_Building_try_offer_unit(pub UnitType, pub ImmigrationManagerID);
#[derive(Compact, Clone)] #[allow(non_camel_case_types)]
struct MSG_Building_add_household(pub HouseholdID, pub UnitIdx);
#[derive(Compact, Clone)] #[allow(non_camel_case_types)]
struct MSG_Building_remove_household(pub HouseholdID);
#[derive(Copy, Clone)] #[allow(non_camel_case_types)]
struct MSG_Building_finally_destroy();
#[derive(Compact, Clone)] #[allow(non_camel_case_types)]
struct MSG_Building_get_ui_info(pub LandUseUIID);
#[derive(Compact, Clone)] #[allow(non_camel_case_types)]
struct MSG_Building_reconnect(pub PreciseLocation, pub P2);

impl Into<ConstructableID> for BuildingID {
    fn into(self) -> ConstructableID {
        ConstructableID::from_raw(self.as_raw())
    }
}

impl Into<AttacheeID> for BuildingID {
    fn into(self) -> AttacheeID {
        AttacheeID::from_raw(self.as_raw())
    }
}

impl Into<SleeperID> for BuildingID {
    fn into(self) -> SleeperID {
        SleeperID::from_raw(self.as_raw())
    }
}

impl Into<RoughLocationID> for BuildingID {
    fn into(self) -> RoughLocationID {
        RoughLocationID::from_raw(self.as_raw())
    }
}

#[allow(unused_variables)]
#[allow(unused_mut)]
pub fn auto_setup(system: &mut ActorSystem) {
    
    ConstructableID::register_implementor::<Building>(system);
    AttacheeID::register_implementor::<Building>(system);
    SleeperID::register_implementor::<Building>(system);
    RoughLocationID::register_implementor::<Building>(system);
    system.add_spawner::<Building, _, _>(
        |&MSG_Building_spawn(id, style, ref lot), world| {
            Building::spawn(id, style, lot, world)
        }, false
    );
    
    system.add_handler::<Building, _, _>(
        |&MSG_Building_try_offer_unit(required_unit_type, requester), instance, world| {
            instance.try_offer_unit(required_unit_type, requester, world); Fate::Live
        }, false
    );
    
    system.add_handler::<Building, _, _>(
        |&MSG_Building_add_household(household, unit), instance, world| {
            instance.add_household(household, unit, world); Fate::Live
        }, false
    );
    
    system.add_handler::<Building, _, _>(
        |&MSG_Building_remove_household(household), instance, world| {
            instance.remove_household(household, world); Fate::Live
        }, false
    );
    
    system.add_handler::<Building, _, _>(
        |&MSG_Building_finally_destroy(), instance, world| {
            instance.finally_destroy(world)
        }, false
    );
    
    system.add_handler::<Building, _, _>(
        |&MSG_Building_get_ui_info(requester), instance, world| {
            instance.get_ui_info(requester, world); Fate::Live
        }, false
    );
    
    system.add_handler::<Building, _, _>(
        |&MSG_Building_reconnect(new_location, new_connection_point), instance, world| {
            instance.reconnect(new_location, new_connection_point, world); Fate::Live
        }, false
    );
}