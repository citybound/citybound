//! This is all auto-generated. Do not touch.
#![rustfmt::skip]
#[allow(unused_imports)]
use kay::{ActorSystem, TypedID, RawID, Fate, Actor, TraitIDFrom, ActorOrActorTrait};
#[allow(unused_imports)]
use super::*;



impl Actor for CarLane {
    type ID = CarLaneID;

    fn id(&self) -> Self::ID {
        self.id
    }
    unsafe fn set_id(&mut self, id: RawID) {
        self.id = Self::ID::from_raw(id);
    }
}

#[derive(Serialize, Deserialize)] #[serde(transparent)]
pub struct CarLaneID {
    _raw_id: RawID
}

impl Copy for CarLaneID {}
impl Clone for CarLaneID { fn clone(&self) -> Self { *self } }
impl ::std::fmt::Debug for CarLaneID {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        write!(f, "CarLaneID({:?})", self._raw_id)
    }
}
impl ::std::hash::Hash for CarLaneID {
    fn hash<H: ::std::hash::Hasher>(&self, state: &mut H) {
        self._raw_id.hash(state);
    }
}
impl PartialEq for CarLaneID {
    fn eq(&self, other: &CarLaneID) -> bool {
        self._raw_id == other._raw_id
    }
}
impl Eq for CarLaneID {}

impl TypedID for CarLaneID {
    type Target = CarLane;

    fn from_raw(id: RawID) -> Self {
        CarLaneID { _raw_id: id }
    }

    fn as_raw(&self) -> RawID {
        self._raw_id
    }
}

impl CarLaneID {
    pub fn spawn(path: LinePath, on_intersection: bool, timings: CVec < bool >, world: &mut World) -> Self {
        let id = CarLaneID::from_raw(world.allocate_instance_id::<CarLane>());
        let swarm = world.local_broadcast::<CarLane>();
        world.send(swarm, MSG_CarLane_spawn(id, path, on_intersection, timings));
        id
    }
}

#[derive(Compact, Clone)] #[allow(non_camel_case_types)]
struct MSG_CarLane_spawn(pub CarLaneID, pub LinePath, pub bool, pub CVec < bool >);


impl Actor for CarSwitchLane {
    type ID = CarSwitchLaneID;

    fn id(&self) -> Self::ID {
        self.id
    }
    unsafe fn set_id(&mut self, id: RawID) {
        self.id = Self::ID::from_raw(id);
    }
}

#[derive(Serialize, Deserialize)] #[serde(transparent)]
pub struct CarSwitchLaneID {
    _raw_id: RawID
}

impl Copy for CarSwitchLaneID {}
impl Clone for CarSwitchLaneID { fn clone(&self) -> Self { *self } }
impl ::std::fmt::Debug for CarSwitchLaneID {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        write!(f, "CarSwitchLaneID({:?})", self._raw_id)
    }
}
impl ::std::hash::Hash for CarSwitchLaneID {
    fn hash<H: ::std::hash::Hasher>(&self, state: &mut H) {
        self._raw_id.hash(state);
    }
}
impl PartialEq for CarSwitchLaneID {
    fn eq(&self, other: &CarSwitchLaneID) -> bool {
        self._raw_id == other._raw_id
    }
}
impl Eq for CarSwitchLaneID {}

impl TypedID for CarSwitchLaneID {
    type Target = CarSwitchLane;

    fn from_raw(id: RawID) -> Self {
        CarSwitchLaneID { _raw_id: id }
    }

    fn as_raw(&self) -> RawID {
        self._raw_id
    }
}

impl CarSwitchLaneID {
    pub fn spawn(path: LinePath, world: &mut World) -> Self {
        let id = CarSwitchLaneID::from_raw(world.allocate_instance_id::<CarSwitchLane>());
        let swarm = world.local_broadcast::<CarSwitchLane>();
        world.send(swarm, MSG_CarSwitchLane_spawn(id, path));
        id
    }
}

#[derive(Compact, Clone)] #[allow(non_camel_case_types)]
struct MSG_CarSwitchLane_spawn(pub CarSwitchLaneID, pub LinePath);


impl Actor for Sidewalk {
    type ID = SidewalkID;

    fn id(&self) -> Self::ID {
        self.id
    }
    unsafe fn set_id(&mut self, id: RawID) {
        self.id = Self::ID::from_raw(id);
    }
}

#[derive(Serialize, Deserialize)] #[serde(transparent)]
pub struct SidewalkID {
    _raw_id: RawID
}

impl Copy for SidewalkID {}
impl Clone for SidewalkID { fn clone(&self) -> Self { *self } }
impl ::std::fmt::Debug for SidewalkID {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        write!(f, "SidewalkID({:?})", self._raw_id)
    }
}
impl ::std::hash::Hash for SidewalkID {
    fn hash<H: ::std::hash::Hasher>(&self, state: &mut H) {
        self._raw_id.hash(state);
    }
}
impl PartialEq for SidewalkID {
    fn eq(&self, other: &SidewalkID) -> bool {
        self._raw_id == other._raw_id
    }
}
impl Eq for SidewalkID {}

impl TypedID for SidewalkID {
    type Target = Sidewalk;

    fn from_raw(id: RawID) -> Self {
        SidewalkID { _raw_id: id }
    }

    fn as_raw(&self) -> RawID {
        self._raw_id
    }
}

impl SidewalkID {
    pub fn spawn(path: LinePath, on_intersection: bool, timings: CVec < bool >, world: &mut World) -> Self {
        let id = SidewalkID::from_raw(world.allocate_instance_id::<Sidewalk>());
        let swarm = world.local_broadcast::<Sidewalk>();
        world.send(swarm, MSG_Sidewalk_spawn(id, path, on_intersection, timings));
        id
    }
}

#[derive(Compact, Clone)] #[allow(non_camel_case_types)]
struct MSG_Sidewalk_spawn(pub SidewalkID, pub LinePath, pub bool, pub CVec < bool >);


#[allow(unused_variables)]
#[allow(unused_mut)]
pub fn auto_setup(system: &mut ActorSystem) {
    
    
    system.add_spawner::<CarLane, _, _>(
        |&MSG_CarLane_spawn(id, ref path, on_intersection, ref timings), world| {
            CarLane::spawn(id, path, on_intersection, timings, world)
        }, false
    );
    
    system.add_spawner::<CarSwitchLane, _, _>(
        |&MSG_CarSwitchLane_spawn(id, ref path), world| {
            CarSwitchLane::spawn(id, path, world)
        }, false
    );
    
    system.add_spawner::<Sidewalk, _, _>(
        |&MSG_Sidewalk_spawn(id, ref path, on_intersection, ref timings), world| {
            Sidewalk::spawn(id, path, on_intersection, timings, world)
        }, false
    );
}