//! This is all auto-generated. Do not touch.
#![cfg_attr(rustfmt, rustfmt_skip)]
#[allow(unused_imports)]
use kay::{ActorSystem, TypedID, RawID, Fate, Actor, TraitIDFrom, ActorOrActorTrait};
#[allow(unused_imports)]
use super::*;



impl Actor for Lane {
    type ID = LaneID;

    fn id(&self) -> Self::ID {
        self.id
    }
    unsafe fn set_id(&mut self, id: RawID) {
        self.id = Self::ID::from_raw(id);
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)] #[serde(transparent)]
pub struct LaneID {
    _raw_id: RawID
}

impl TypedID for LaneID {
    type Target = Lane;

    fn from_raw(id: RawID) -> Self {
        LaneID { _raw_id: id }
    }

    fn as_raw(&self) -> RawID {
        self._raw_id
    }
}

impl LaneID {
    pub fn spawn(path: LinePath, on_intersection: bool, timings: CVec < bool >, world: &mut World) -> Self {
        let id = LaneID::from_raw(world.allocate_instance_id::<Lane>());
        let swarm = world.local_broadcast::<Lane>();
        world.send(swarm, MSG_Lane_spawn(id, path, on_intersection, timings));
        id
    }
}

#[derive(Compact, Clone)] #[allow(non_camel_case_types)]
struct MSG_Lane_spawn(pub LaneID, pub LinePath, pub bool, pub CVec < bool >);


impl Actor for SwitchLane {
    type ID = SwitchLaneID;

    fn id(&self) -> Self::ID {
        self.id
    }
    unsafe fn set_id(&mut self, id: RawID) {
        self.id = Self::ID::from_raw(id);
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)] #[serde(transparent)]
pub struct SwitchLaneID {
    _raw_id: RawID
}

impl TypedID for SwitchLaneID {
    type Target = SwitchLane;

    fn from_raw(id: RawID) -> Self {
        SwitchLaneID { _raw_id: id }
    }

    fn as_raw(&self) -> RawID {
        self._raw_id
    }
}

impl SwitchLaneID {
    pub fn spawn(path: LinePath, world: &mut World) -> Self {
        let id = SwitchLaneID::from_raw(world.allocate_instance_id::<SwitchLane>());
        let swarm = world.local_broadcast::<SwitchLane>();
        world.send(swarm, MSG_SwitchLane_spawn(id, path));
        id
    }
}

#[derive(Compact, Clone)] #[allow(non_camel_case_types)]
struct MSG_SwitchLane_spawn(pub SwitchLaneID, pub LinePath);


#[allow(unused_variables)]
#[allow(unused_mut)]
pub fn auto_setup(system: &mut ActorSystem) {
    
    
    system.add_spawner::<Lane, _, _>(
        |&MSG_Lane_spawn(id, ref path, on_intersection, ref timings), world| {
            Lane::spawn(id, path, on_intersection, timings, world)
        }, false
    );
    
    system.add_spawner::<SwitchLane, _, _>(
        |&MSG_SwitchLane_spawn(id, ref path), world| {
            SwitchLane::spawn(id, path, world)
        }, false
    );
}