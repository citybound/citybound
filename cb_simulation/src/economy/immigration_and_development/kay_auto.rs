//! This is all auto-generated. Do not touch.
#![rustfmt::skip]
#[allow(unused_imports)]
use kay::{ActorSystem, TypedID, RawID, Fate, Actor, TraitIDFrom, ActorOrActorTrait};
#[allow(unused_imports)]
use super::*;



impl Actor for ImmigrationManager {
    type ID = ImmigrationManagerID;

    fn id(&self) -> Self::ID {
        self.id
    }
    unsafe fn set_id(&mut self, id: RawID) {
        self.id = Self::ID::from_raw(id);
    }
}

#[derive(Serialize, Deserialize)] #[serde(transparent)]
pub struct ImmigrationManagerID {
    _raw_id: RawID
}

impl Copy for ImmigrationManagerID {}
impl Clone for ImmigrationManagerID { fn clone(&self) -> Self { *self } }
impl ::std::fmt::Debug for ImmigrationManagerID {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        write!(f, "ImmigrationManagerID({:?})", self._raw_id)
    }
}
impl ::std::hash::Hash for ImmigrationManagerID {
    fn hash<H: ::std::hash::Hasher>(&self, state: &mut H) {
        self._raw_id.hash(state);
    }
}
impl PartialEq for ImmigrationManagerID {
    fn eq(&self, other: &ImmigrationManagerID) -> bool {
        self._raw_id == other._raw_id
    }
}
impl Eq for ImmigrationManagerID {}

impl TypedID for ImmigrationManagerID {
    type Target = ImmigrationManager;

    fn from_raw(id: RawID) -> Self {
        ImmigrationManagerID { _raw_id: id }
    }

    fn as_raw(&self) -> RawID {
        self._raw_id
    }
}

impl ImmigrationManagerID {
    pub fn spawn(time: TimeID, development_manager: DevelopmentManagerID, world: &mut World) -> Self {
        let id = ImmigrationManagerID::from_raw(world.allocate_instance_id::<ImmigrationManager>());
        let swarm = world.local_broadcast::<ImmigrationManager>();
        world.send(swarm, MSG_ImmigrationManager_spawn(id, time, development_manager));
        id
    }
    
    pub fn on_unit_offer(self, building_id: BuildingID, unit_idx: UnitIdx, world: &mut World) {
        world.send(self.as_raw(), MSG_ImmigrationManager_on_unit_offer(building_id, unit_idx));
    }
}

#[derive(Compact, Clone)] #[allow(non_camel_case_types)]
struct MSG_ImmigrationManager_spawn(pub ImmigrationManagerID, pub TimeID, pub DevelopmentManagerID);
#[derive(Compact, Clone)] #[allow(non_camel_case_types)]
struct MSG_ImmigrationManager_on_unit_offer(pub BuildingID, pub UnitIdx);

impl Into<SleeperID> for ImmigrationManagerID {
    fn into(self) -> SleeperID {
        SleeperID::from_raw(self.as_raw())
    }
}
impl Actor for DevelopmentManager {
    type ID = DevelopmentManagerID;

    fn id(&self) -> Self::ID {
        self.id
    }
    unsafe fn set_id(&mut self, id: RawID) {
        self.id = Self::ID::from_raw(id);
    }
}

#[derive(Serialize, Deserialize)] #[serde(transparent)]
pub struct DevelopmentManagerID {
    _raw_id: RawID
}

impl Copy for DevelopmentManagerID {}
impl Clone for DevelopmentManagerID { fn clone(&self) -> Self { *self } }
impl ::std::fmt::Debug for DevelopmentManagerID {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        write!(f, "DevelopmentManagerID({:?})", self._raw_id)
    }
}
impl ::std::hash::Hash for DevelopmentManagerID {
    fn hash<H: ::std::hash::Hasher>(&self, state: &mut H) {
        self._raw_id.hash(state);
    }
}
impl PartialEq for DevelopmentManagerID {
    fn eq(&self, other: &DevelopmentManagerID) -> bool {
        self._raw_id == other._raw_id
    }
}
impl Eq for DevelopmentManagerID {}

impl TypedID for DevelopmentManagerID {
    type Target = DevelopmentManager;

    fn from_raw(id: RawID) -> Self {
        DevelopmentManagerID { _raw_id: id }
    }

    fn as_raw(&self) -> RawID {
        self._raw_id
    }
}

impl DevelopmentManagerID {
    pub fn spawn(time: TimeID, plan_manager: CBPlanManagerID, world: &mut World) -> Self {
        let id = DevelopmentManagerID::from_raw(world.allocate_instance_id::<DevelopmentManager>());
        let swarm = world.local_broadcast::<DevelopmentManager>();
        world.send(swarm, MSG_DevelopmentManager_spawn(id, time, plan_manager));
        id
    }
    
    pub fn try_develop(self, building_style: BuildingStyle, world: &mut World) {
        world.send(self.as_raw(), MSG_DevelopmentManager_try_develop(building_style));
    }
    
    pub fn on_suggested_lot(self, building_intent: BuildingIntent, based_on: PrototypeID, world: &mut World) {
        world.send(self.as_raw(), MSG_DevelopmentManager_on_suggested_lot(building_intent, based_on));
    }
}

#[derive(Compact, Clone)] #[allow(non_camel_case_types)]
struct MSG_DevelopmentManager_spawn(pub DevelopmentManagerID, pub TimeID, pub CBPlanManagerID);
#[derive(Compact, Clone)] #[allow(non_camel_case_types)]
struct MSG_DevelopmentManager_try_develop(pub BuildingStyle);
#[derive(Compact, Clone)] #[allow(non_camel_case_types)]
struct MSG_DevelopmentManager_on_suggested_lot(pub BuildingIntent, pub PrototypeID);

impl Into<SleeperID> for DevelopmentManagerID {
    fn into(self) -> SleeperID {
        SleeperID::from_raw(self.as_raw())
    }
}

#[allow(unused_variables)]
#[allow(unused_mut)]
pub fn auto_setup(system: &mut ActorSystem) {
    
    SleeperID::register_implementor::<ImmigrationManager>(system);
    system.add_spawner::<ImmigrationManager, _, _>(
        |&MSG_ImmigrationManager_spawn(id, time, development_manager), world| {
            ImmigrationManager::spawn(id, time, development_manager, world)
        }, false
    );
    
    system.add_handler::<ImmigrationManager, _, _>(
        |&MSG_ImmigrationManager_on_unit_offer(building_id, unit_idx), instance, world| {
            instance.on_unit_offer(building_id, unit_idx, world); Fate::Live
        }, false
    );
    SleeperID::register_implementor::<DevelopmentManager>(system);
    system.add_spawner::<DevelopmentManager, _, _>(
        |&MSG_DevelopmentManager_spawn(id, time, plan_manager), world| {
            DevelopmentManager::spawn(id, time, plan_manager, world)
        }, false
    );
    
    system.add_handler::<DevelopmentManager, _, _>(
        |&MSG_DevelopmentManager_try_develop(building_style), instance, world| {
            instance.try_develop(building_style, world); Fate::Live
        }, false
    );
    
    system.add_handler::<DevelopmentManager, _, _>(
        |&MSG_DevelopmentManager_on_suggested_lot(ref building_intent, based_on), instance, world| {
            instance.on_suggested_lot(building_intent, based_on, world); Fate::Live
        }, false
    );
}