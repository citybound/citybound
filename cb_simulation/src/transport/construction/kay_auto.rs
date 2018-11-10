//! This is all auto-generated. Do not touch.
#![cfg_attr(rustfmt, rustfmt_skip)]
#[allow(unused_imports)]
use kay::{ActorSystem, TypedID, RawID, Fate, Actor, TraitIDFrom, ActorOrActorTrait};
#[allow(unused_imports)]
use super::*;

#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)] #[serde(transparent)]
pub struct UnbuildableID {
    _raw_id: RawID
}

pub struct UnbuildableRepresentative;

impl ActorOrActorTrait for UnbuildableRepresentative {
    type ID = UnbuildableID;
}

impl TypedID for UnbuildableID {
    type Target = UnbuildableRepresentative;

    fn from_raw(id: RawID) -> Self {
        UnbuildableID { _raw_id: id }
    }

    fn as_raw(&self) -> RawID {
        self._raw_id
    }
}

impl<A: Actor + Unbuildable> TraitIDFrom<A> for UnbuildableID {}

impl UnbuildableID {
    pub fn disconnect(&self, other_id: UnbuildableID, world: &mut World) {
        world.send(self.as_raw(), MSG_Unbuildable_disconnect(other_id));
    }
    
    pub fn unbuild(&self, report_to: ConstructionID, world: &mut World) {
        world.send(self.as_raw(), MSG_Unbuildable_unbuild(report_to));
    }
    
    pub fn on_confirm_disconnect(&self, world: &mut World) {
        world.send(self.as_raw(), MSG_Unbuildable_on_confirm_disconnect());
    }

    pub fn register_trait(system: &mut ActorSystem) {
        system.register_trait::<UnbuildableRepresentative>();
        system.register_trait_message::<MSG_Unbuildable_disconnect>();
        system.register_trait_message::<MSG_Unbuildable_unbuild>();
        system.register_trait_message::<MSG_Unbuildable_on_confirm_disconnect>();
    }

    pub fn register_implementor<A: Actor + Unbuildable>(system: &mut ActorSystem) {
        system.register_implementor::<A, UnbuildableRepresentative>();
        system.add_handler::<A, _, _>(
            |&MSG_Unbuildable_disconnect(other_id), instance, world| {
                instance.disconnect(other_id, world); Fate::Live
            }, false
        );
        
        system.add_handler::<A, _, _>(
            |&MSG_Unbuildable_unbuild(report_to), instance, world| {
                instance.unbuild(report_to, world)
            }, false
        );
        
        system.add_handler::<A, _, _>(
            |&MSG_Unbuildable_on_confirm_disconnect(), instance, world| {
                instance.on_confirm_disconnect(world)
            }, false
        );
    }
}

#[derive(Compact, Clone)] #[allow(non_camel_case_types)]
struct MSG_Unbuildable_disconnect(pub UnbuildableID);
#[derive(Compact, Clone)] #[allow(non_camel_case_types)]
struct MSG_Unbuildable_unbuild(pub ConstructionID);
#[derive(Copy, Clone)] #[allow(non_camel_case_types)]
struct MSG_Unbuildable_on_confirm_disconnect();



impl LaneID {
    pub fn spawn_and_connect(path: LinePath, on_intersection: bool, timings: CVec < bool >, report_to: ConstructionID, world: &mut World) -> Self {
        let id = LaneID::from_raw(world.allocate_instance_id::<Lane>());
        let swarm = world.local_broadcast::<Lane>();
        world.send(swarm, MSG_Lane_spawn_and_connect(id, path, on_intersection, timings, report_to));
        id
    }
    
    pub fn start_connecting_overlaps(&self, lanes: CVec < LaneID >, world: &mut World) {
        world.send(self.as_raw(), MSG_Lane_start_connecting_overlaps(lanes));
    }
    
    pub fn connect(&self, other_id: LaneID, other_start: P2, other_end: P2, other_length: N, reply_needed: bool, world: &mut World) {
        world.send(self.as_raw(), MSG_Lane_connect(other_id, other_start, other_end, other_length, reply_needed));
    }
    
    pub fn connect_overlaps(&self, other_id: LaneID, other_path: LinePath, reply_needed: bool, world: &mut World) {
        world.send(self.as_raw(), MSG_Lane_connect_overlaps(other_id, other_path, reply_needed));
    }
    
    pub fn connect_to_switch(&self, other_id: SwitchLaneID, world: &mut World) {
        world.send(self.as_raw(), MSG_Lane_connect_to_switch(other_id));
    }
    
    pub fn add_switch_lane_interaction(&self, interaction: Interaction, world: &mut World) {
        world.send(self.as_raw(), MSG_Lane_add_switch_lane_interaction(interaction));
    }
    
    pub fn try_reconnect_building(&self, building: BuildingID, lot_position: P2, world: &mut World) {
        world.send(self.as_raw(), MSG_Lane_try_reconnect_building(building, lot_position));
    }
}

#[derive(Compact, Clone)] #[allow(non_camel_case_types)]
struct MSG_Lane_spawn_and_connect(pub LaneID, pub LinePath, pub bool, pub CVec < bool >, pub ConstructionID);
#[derive(Compact, Clone)] #[allow(non_camel_case_types)]
struct MSG_Lane_start_connecting_overlaps(pub CVec < LaneID >);
#[derive(Compact, Clone)] #[allow(non_camel_case_types)]
struct MSG_Lane_connect(pub LaneID, pub P2, pub P2, pub N, pub bool);
#[derive(Compact, Clone)] #[allow(non_camel_case_types)]
struct MSG_Lane_connect_overlaps(pub LaneID, pub LinePath, pub bool);
#[derive(Compact, Clone)] #[allow(non_camel_case_types)]
struct MSG_Lane_connect_to_switch(pub SwitchLaneID);
#[derive(Compact, Clone)] #[allow(non_camel_case_types)]
struct MSG_Lane_add_switch_lane_interaction(pub Interaction);
#[derive(Compact, Clone)] #[allow(non_camel_case_types)]
struct MSG_Lane_try_reconnect_building(pub BuildingID, pub P2);

impl Into<ConstructableID> for LaneID {
    fn into(self) -> ConstructableID {
        ConstructableID::from_raw(self.as_raw())
    }
}

impl Into<UnbuildableID> for LaneID {
    fn into(self) -> UnbuildableID {
        UnbuildableID::from_raw(self.as_raw())
    }
}


impl SwitchLaneID {
    pub fn spawn_and_connect(path: LinePath, report_to: ConstructionID, world: &mut World) -> Self {
        let id = SwitchLaneID::from_raw(world.allocate_instance_id::<SwitchLane>());
        let swarm = world.local_broadcast::<SwitchLane>();
        world.send(swarm, MSG_SwitchLane_spawn_and_connect(id, path, report_to));
        id
    }
    
    pub fn connect_switch_to_normal(&self, other_id: LaneID, other_path: LinePath, world: &mut World) {
        world.send(self.as_raw(), MSG_SwitchLane_connect_switch_to_normal(other_id, other_path));
    }
}

#[derive(Compact, Clone)] #[allow(non_camel_case_types)]
struct MSG_SwitchLane_spawn_and_connect(pub SwitchLaneID, pub LinePath, pub ConstructionID);
#[derive(Compact, Clone)] #[allow(non_camel_case_types)]
struct MSG_SwitchLane_connect_switch_to_normal(pub LaneID, pub LinePath);

impl Into<ConstructableID> for SwitchLaneID {
    fn into(self) -> ConstructableID {
        ConstructableID::from_raw(self.as_raw())
    }
}

impl Into<UnbuildableID> for SwitchLaneID {
    fn into(self) -> UnbuildableID {
        UnbuildableID::from_raw(self.as_raw())
    }
}

#[allow(unused_variables)]
#[allow(unused_mut)]
pub fn auto_setup(system: &mut ActorSystem) {
    UnbuildableID::register_trait(system);
    ConstructableID::register_implementor::<Lane>(system);
    UnbuildableID::register_implementor::<Lane>(system);
    system.add_spawner::<Lane, _, _>(
        |&MSG_Lane_spawn_and_connect(id, ref path, on_intersection, ref timings, report_to), world| {
            Lane::spawn_and_connect(id, path, on_intersection, timings, report_to, world)
        }, false
    );
    
    system.add_handler::<Lane, _, _>(
        |&MSG_Lane_start_connecting_overlaps(ref lanes), instance, world| {
            instance.start_connecting_overlaps(lanes, world); Fate::Live
        }, false
    );
    
    system.add_handler::<Lane, _, _>(
        |&MSG_Lane_connect(other_id, other_start, other_end, other_length, reply_needed), instance, world| {
            instance.connect(other_id, other_start, other_end, other_length, reply_needed, world); Fate::Live
        }, false
    );
    
    system.add_handler::<Lane, _, _>(
        |&MSG_Lane_connect_overlaps(other_id, ref other_path, reply_needed), instance, world| {
            instance.connect_overlaps(other_id, other_path, reply_needed, world); Fate::Live
        }, false
    );
    
    system.add_handler::<Lane, _, _>(
        |&MSG_Lane_connect_to_switch(other_id), instance, world| {
            instance.connect_to_switch(other_id, world); Fate::Live
        }, false
    );
    
    system.add_handler::<Lane, _, _>(
        |&MSG_Lane_add_switch_lane_interaction(interaction), instance, world| {
            instance.add_switch_lane_interaction(interaction, world); Fate::Live
        }, false
    );
    
    system.add_handler::<Lane, _, _>(
        |&MSG_Lane_try_reconnect_building(building, lot_position), instance, world| {
            instance.try_reconnect_building(building, lot_position, world); Fate::Live
        }, false
    );
    ConstructableID::register_implementor::<SwitchLane>(system);
    UnbuildableID::register_implementor::<SwitchLane>(system);
    system.add_spawner::<SwitchLane, _, _>(
        |&MSG_SwitchLane_spawn_and_connect(id, ref path, report_to), world| {
            SwitchLane::spawn_and_connect(id, path, report_to, world)
        }, false
    );
    
    system.add_handler::<SwitchLane, _, _>(
        |&MSG_SwitchLane_connect_switch_to_normal(other_id, ref other_path), instance, world| {
            instance.connect_switch_to_normal(other_id, other_path, world); Fate::Live
        }, false
    );
}