//! This is all auto-generated. Do not touch.
#![rustfmt::skip]
#[allow(unused_imports)]
use kay::{ActorSystem, TypedID, RawID, Fate, Actor, TraitIDFrom, ActorOrActorTrait};
#[allow(unused_imports)]
use super::*;





impl CarLaneID {
    pub fn spawn_and_connect(path: LinePath, on_intersection: bool, timings: CVec < bool >, report_to: CBConstructionID, world: &mut World) -> Self {
        let id = CarLaneID::from_raw(world.allocate_instance_id::<CarLane>());
        let swarm = world.local_broadcast::<CarLane>();
        world.send(swarm, MSG_CarLane_spawn_and_connect(id, path, on_intersection, timings, report_to));
        id
    }
    
    pub fn connect(self, other_id: CarLaneID, other_start: P2, other_end: P2, other_length: N, reply_needed: bool, world: &mut World) {
        world.send(self.as_raw(), MSG_CarLane_connect(other_id, other_start, other_end, other_length, reply_needed));
    }
    
    pub fn connect_to_switch(self, other_id: CarSwitchLaneID, world: &mut World) {
        world.send(self.as_raw(), MSG_CarLane_connect_to_switch(other_id));
    }
    
    pub fn add_switch_lane_interaction(self, interaction: CarLaneInteraction, world: &mut World) {
        world.send(self.as_raw(), MSG_CarLane_add_switch_lane_interaction(interaction));
    }
    
    pub fn disconnect_switch(self, other_id: CarSwitchLaneID, world: &mut World) {
        world.send(self.as_raw(), MSG_CarLane_disconnect_switch(other_id));
    }
    
    pub fn unbuild(self, report_to: CBConstructionID, world: &mut World) {
        world.send(self.as_raw(), MSG_CarLane_unbuild(report_to));
    }
    
    pub fn finalize(self, report_to: CBConstructionID, world: &mut World) {
        world.send(self.as_raw(), MSG_CarLane_finalize(report_to));
    }
    
    pub fn try_reconnect_building(self, building: BuildingID, lot_position: P2, world: &mut World) {
        world.send(self.as_raw(), MSG_CarLane_try_reconnect_building(building, lot_position));
    }
}

#[derive(Compact, Clone)] #[allow(non_camel_case_types)]
struct MSG_CarLane_spawn_and_connect(pub CarLaneID, pub LinePath, pub bool, pub CVec < bool >, pub CBConstructionID);
#[derive(Compact, Clone)] #[allow(non_camel_case_types)]
struct MSG_CarLane_connect(pub CarLaneID, pub P2, pub P2, pub N, pub bool);
#[derive(Compact, Clone)] #[allow(non_camel_case_types)]
struct MSG_CarLane_connect_to_switch(pub CarSwitchLaneID);
#[derive(Compact, Clone)] #[allow(non_camel_case_types)]
struct MSG_CarLane_add_switch_lane_interaction(pub CarLaneInteraction);
#[derive(Compact, Clone)] #[allow(non_camel_case_types)]
struct MSG_CarLane_disconnect_switch(pub CarSwitchLaneID);
#[derive(Compact, Clone)] #[allow(non_camel_case_types)]
struct MSG_CarLane_unbuild(pub CBConstructionID);
#[derive(Compact, Clone)] #[allow(non_camel_case_types)]
struct MSG_CarLane_finalize(pub CBConstructionID);
#[derive(Compact, Clone)] #[allow(non_camel_case_types)]
struct MSG_CarLane_try_reconnect_building(pub BuildingID, pub P2);

impl Into<ConstructableID<CBPrototypeKind>> for CarLaneID {
    fn into(self) -> ConstructableID<CBPrototypeKind> {
        ConstructableID::from_raw(self.as_raw())
    }
}


impl CarSwitchLaneID {
    pub fn spawn_and_connect(path: LinePath, report_to: CBConstructionID, world: &mut World) -> Self {
        let id = CarSwitchLaneID::from_raw(world.allocate_instance_id::<CarSwitchLane>());
        let swarm = world.local_broadcast::<CarSwitchLane>();
        world.send(swarm, MSG_CarSwitchLane_spawn_and_connect(id, path, report_to));
        id
    }
    
    pub fn unbuild(self, report_to: CBConstructionID, world: &mut World) {
        world.send(self.as_raw(), MSG_CarSwitchLane_unbuild(report_to));
    }
    
    pub fn finalize(self, report_to: CBConstructionID, world: &mut World) {
        world.send(self.as_raw(), MSG_CarSwitchLane_finalize(report_to));
    }
}

#[derive(Compact, Clone)] #[allow(non_camel_case_types)]
struct MSG_CarSwitchLane_spawn_and_connect(pub CarSwitchLaneID, pub LinePath, pub CBConstructionID);
#[derive(Compact, Clone)] #[allow(non_camel_case_types)]
struct MSG_CarSwitchLane_unbuild(pub CBConstructionID);
#[derive(Compact, Clone)] #[allow(non_camel_case_types)]
struct MSG_CarSwitchLane_finalize(pub CBConstructionID);

impl Into<ConstructableID<CBPrototypeKind>> for CarSwitchLaneID {
    fn into(self) -> ConstructableID<CBPrototypeKind> {
        ConstructableID::from_raw(self.as_raw())
    }
}


impl SidewalkID {
    pub fn spawn_and_connect(path: LinePath, on_intersection: bool, timings: CVec < bool >, report_to: CBConstructionID, world: &mut World) -> Self {
        let id = SidewalkID::from_raw(world.allocate_instance_id::<Sidewalk>());
        let swarm = world.local_broadcast::<Sidewalk>();
        world.send(swarm, MSG_Sidewalk_spawn_and_connect(id, path, on_intersection, timings, report_to));
        id
    }
    
    pub fn connect(self, other_id: SidewalkID, other_start: P2, other_end: P2, other_length: N, reply_needed: bool, world: &mut World) {
        world.send(self.as_raw(), MSG_Sidewalk_connect(other_id, other_start, other_end, other_length, reply_needed));
    }
    
    pub fn finalize(self, report_to: CBConstructionID, world: &mut World) {
        world.send(self.as_raw(), MSG_Sidewalk_finalize(report_to));
    }
}

#[derive(Compact, Clone)] #[allow(non_camel_case_types)]
struct MSG_Sidewalk_spawn_and_connect(pub SidewalkID, pub LinePath, pub bool, pub CVec < bool >, pub CBConstructionID);
#[derive(Compact, Clone)] #[allow(non_camel_case_types)]
struct MSG_Sidewalk_connect(pub SidewalkID, pub P2, pub P2, pub N, pub bool);
#[derive(Compact, Clone)] #[allow(non_camel_case_types)]
struct MSG_Sidewalk_finalize(pub CBConstructionID);

impl Into<ConstructableID<CBPrototypeKind>> for SidewalkID {
    fn into(self) -> ConstructableID<CBPrototypeKind> {
        ConstructableID::from_raw(self.as_raw())
    }
}

#[allow(unused_variables)]
#[allow(unused_mut)]
pub fn auto_setup(system: &mut ActorSystem) {
    
    ConstructableID::<CBPrototypeKind>::register_implementor::<CarLane>(system);
    system.add_spawner::<CarLane, _, _>(
        |&MSG_CarLane_spawn_and_connect(id, ref path, on_intersection, ref timings, report_to), world| {
            CarLane::spawn_and_connect(id, path, on_intersection, timings, report_to, world)
        }, false
    );
    
    system.add_handler::<CarLane, _, _>(
        |&MSG_CarLane_connect(other_id, other_start, other_end, other_length, reply_needed), instance, world| {
            instance.connect(other_id, other_start, other_end, other_length, reply_needed, world); Fate::Live
        }, false
    );
    
    system.add_handler::<CarLane, _, _>(
        |&MSG_CarLane_connect_to_switch(other_id), instance, world| {
            instance.connect_to_switch(other_id, world); Fate::Live
        }, false
    );
    
    system.add_handler::<CarLane, _, _>(
        |&MSG_CarLane_add_switch_lane_interaction(interaction), instance, world| {
            instance.add_switch_lane_interaction(interaction, world); Fate::Live
        }, false
    );
    
    system.add_handler::<CarLane, _, _>(
        |&MSG_CarLane_disconnect_switch(other_id), instance, world| {
            instance.disconnect_switch(other_id, world); Fate::Live
        }, false
    );
    
    system.add_handler::<CarLane, _, _>(
        |&MSG_CarLane_unbuild(report_to), instance, world| {
            instance.unbuild(report_to, world)
        }, false
    );
    
    system.add_handler::<CarLane, _, _>(
        |&MSG_CarLane_finalize(report_to), instance, world| {
            instance.finalize(report_to, world); Fate::Live
        }, false
    );
    
    system.add_handler::<CarLane, _, _>(
        |&MSG_CarLane_try_reconnect_building(building, lot_position), instance, world| {
            instance.try_reconnect_building(building, lot_position, world); Fate::Live
        }, false
    );
    ConstructableID::<CBPrototypeKind>::register_implementor::<CarSwitchLane>(system);
    system.add_spawner::<CarSwitchLane, _, _>(
        |&MSG_CarSwitchLane_spawn_and_connect(id, ref path, report_to), world| {
            CarSwitchLane::spawn_and_connect(id, path, report_to, world)
        }, false
    );
    
    system.add_handler::<CarSwitchLane, _, _>(
        |&MSG_CarSwitchLane_unbuild(report_to), instance, world| {
            instance.unbuild(report_to, world)
        }, false
    );
    
    system.add_handler::<CarSwitchLane, _, _>(
        |&MSG_CarSwitchLane_finalize(report_to), instance, world| {
            instance.finalize(report_to, world); Fate::Live
        }, false
    );
    ConstructableID::<CBPrototypeKind>::register_implementor::<Sidewalk>(system);
    system.add_spawner::<Sidewalk, _, _>(
        |&MSG_Sidewalk_spawn_and_connect(id, ref path, on_intersection, ref timings, report_to), world| {
            Sidewalk::spawn_and_connect(id, path, on_intersection, timings, report_to, world)
        }, false
    );
    
    system.add_handler::<Sidewalk, _, _>(
        |&MSG_Sidewalk_connect(other_id, other_start, other_end, other_length, reply_needed), instance, world| {
            instance.connect(other_id, other_start, other_end, other_length, reply_needed, world); Fate::Live
        }, false
    );
    
    system.add_handler::<Sidewalk, _, _>(
        |&MSG_Sidewalk_finalize(report_to), instance, world| {
            instance.finalize(report_to, world); Fate::Live
        }, false
    );
}