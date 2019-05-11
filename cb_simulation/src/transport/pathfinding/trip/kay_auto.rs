//! This is all auto-generated. Do not touch.
#![rustfmt::skip]
#[allow(unused_imports)]
use kay::{ActorSystem, TypedID, RawID, Fate, Actor, TraitIDFrom, ActorOrActorTrait};
#[allow(unused_imports)]
use super::*;

#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)] #[serde(transparent)]
pub struct TripListenerID {
    _raw_id: RawID
}

pub struct TripListenerRepresentative;

impl ActorOrActorTrait for TripListenerRepresentative {
    type ID = TripListenerID;
}

impl TypedID for TripListenerID {
    type Target = TripListenerRepresentative;

    fn from_raw(id: RawID) -> Self {
        TripListenerID { _raw_id: id }
    }

    fn as_raw(&self) -> RawID {
        self._raw_id
    }
}

impl<A: Actor + TripListener> TraitIDFrom<A> for TripListenerID {}

impl TripListenerID {
    pub fn trip_created(self, trip: TripID, world: &mut World) {
        world.send(self.as_raw(), MSG_TripListener_trip_created(trip));
    }
    
    pub fn trip_result(self, trip: TripID, result: TripResult, rough_source: RoughLocationID, rough_destination: RoughLocationID, world: &mut World) {
        world.send(self.as_raw(), MSG_TripListener_trip_result(trip, result, rough_source, rough_destination));
    }

    pub fn register_trait(system: &mut ActorSystem) {
        system.register_trait::<TripListenerRepresentative>();
        system.register_trait_message::<MSG_TripListener_trip_created>();
        system.register_trait_message::<MSG_TripListener_trip_result>();
    }

    pub fn register_implementor<A: Actor + TripListener>(system: &mut ActorSystem) {
        system.register_implementor::<A, TripListenerRepresentative>();
        system.add_handler::<A, _, _>(
            |&MSG_TripListener_trip_created(trip), instance, world| {
                instance.trip_created(trip, world); Fate::Live
            }, false
        );
        
        system.add_handler::<A, _, _>(
            |&MSG_TripListener_trip_result(trip, result, rough_source, rough_destination), instance, world| {
                instance.trip_result(trip, result, rough_source, rough_destination, world); Fate::Live
            }, false
        );
    }
}

#[derive(Compact, Clone)] #[allow(non_camel_case_types)]
struct MSG_TripListener_trip_created(pub TripID);
#[derive(Compact, Clone)] #[allow(non_camel_case_types)]
struct MSG_TripListener_trip_result(pub TripID, pub TripResult, pub RoughLocationID, pub RoughLocationID);

impl Actor for Trip {
    type ID = TripID;

    fn id(&self) -> Self::ID {
        self.id
    }
    unsafe fn set_id(&mut self, id: RawID) {
        self.id = Self::ID::from_raw(id);
    }
}

#[derive(Serialize, Deserialize)] #[serde(transparent)]
pub struct TripID {
    _raw_id: RawID
}

impl Copy for TripID {}
impl Clone for TripID { fn clone(&self) -> Self { *self } }
impl ::std::fmt::Debug for TripID {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        write!(f, "TripID({:?})", self._raw_id)
    }
}
impl ::std::hash::Hash for TripID {
    fn hash<H: ::std::hash::Hasher>(&self, state: &mut H) {
        self._raw_id.hash(state);
    }
}
impl PartialEq for TripID {
    fn eq(&self, other: &TripID) -> bool {
        self._raw_id == other._raw_id
    }
}
impl Eq for TripID {}

impl TypedID for TripID {
    type Target = Trip;

    fn from_raw(id: RawID) -> Self {
        TripID { _raw_id: id }
    }

    fn as_raw(&self) -> RawID {
        self._raw_id
    }
}

impl TripID {
    pub fn spawn(rough_source: RoughLocationID, rough_destination: RoughLocationID, listener: Option < TripListenerID >, instant: Instant, world: &mut World) -> Self {
        let id = TripID::from_raw(world.allocate_instance_id::<Trip>());
        let swarm = world.local_broadcast::<Trip>();
        world.send(swarm, MSG_Trip_spawn(id, rough_source, rough_destination, listener, instant));
        id
    }
    
    pub fn finish(self, result: TripResult, world: &mut World) {
        world.send(self.as_raw(), MSG_Trip_finish(result));
    }
}

#[derive(Compact, Clone)] #[allow(non_camel_case_types)]
struct MSG_Trip_spawn(pub TripID, pub RoughLocationID, pub RoughLocationID, pub Option < TripListenerID >, pub Instant);
#[derive(Compact, Clone)] #[allow(non_camel_case_types)]
struct MSG_Trip_finish(pub TripResult);

impl Into<LocationRequesterID> for TripID {
    fn into(self) -> LocationRequesterID {
        LocationRequesterID::from_raw(self.as_raw())
    }
}
impl Actor for TripCreator {
    type ID = TripCreatorID;

    fn id(&self) -> Self::ID {
        self.id
    }
    unsafe fn set_id(&mut self, id: RawID) {
        self.id = Self::ID::from_raw(id);
    }
}

#[derive(Serialize, Deserialize)] #[serde(transparent)]
pub struct TripCreatorID {
    _raw_id: RawID
}

impl Copy for TripCreatorID {}
impl Clone for TripCreatorID { fn clone(&self) -> Self { *self } }
impl ::std::fmt::Debug for TripCreatorID {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        write!(f, "TripCreatorID({:?})", self._raw_id)
    }
}
impl ::std::hash::Hash for TripCreatorID {
    fn hash<H: ::std::hash::Hasher>(&self, state: &mut H) {
        self._raw_id.hash(state);
    }
}
impl PartialEq for TripCreatorID {
    fn eq(&self, other: &TripCreatorID) -> bool {
        self._raw_id == other._raw_id
    }
}
impl Eq for TripCreatorID {}

impl TypedID for TripCreatorID {
    type Target = TripCreator;

    fn from_raw(id: RawID) -> Self {
        TripCreatorID { _raw_id: id }
    }

    fn as_raw(&self) -> RawID {
        self._raw_id
    }
}

impl TripCreatorID {
    pub fn spawn(time: TimeID, world: &mut World) -> Self {
        let id = TripCreatorID::from_raw(world.allocate_instance_id::<TripCreator>());
        let swarm = world.local_broadcast::<TripCreator>();
        world.send(swarm, MSG_TripCreator_spawn(id, time));
        id
    }
    
    pub fn add_lane_for_trip(self, lane_id: LaneID, world: &mut World) {
        world.send(self.as_raw(), MSG_TripCreator_add_lane_for_trip(lane_id));
    }
}

#[derive(Compact, Clone)] #[allow(non_camel_case_types)]
struct MSG_TripCreator_spawn(pub TripCreatorID, pub TimeID);
#[derive(Compact, Clone)] #[allow(non_camel_case_types)]
struct MSG_TripCreator_add_lane_for_trip(pub LaneID);

impl Into<SleeperID> for TripCreatorID {
    fn into(self) -> SleeperID {
        SleeperID::from_raw(self.as_raw())
    }
}


impl LaneID {
    pub fn manually_spawn_car_add_lane(self, world: &mut World) {
        world.send(self.as_raw(), MSG_Lane_manually_spawn_car_add_lane());
    }
}

#[derive(Copy, Clone)] #[allow(non_camel_case_types)]
struct MSG_Lane_manually_spawn_car_add_lane();


impl Actor for FailedTripDebugger {
    type ID = FailedTripDebuggerID;

    fn id(&self) -> Self::ID {
        self.id
    }
    unsafe fn set_id(&mut self, id: RawID) {
        self.id = Self::ID::from_raw(id);
    }
}

#[derive(Serialize, Deserialize)] #[serde(transparent)]
pub struct FailedTripDebuggerID {
    _raw_id: RawID
}

impl Copy for FailedTripDebuggerID {}
impl Clone for FailedTripDebuggerID { fn clone(&self) -> Self { *self } }
impl ::std::fmt::Debug for FailedTripDebuggerID {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        write!(f, "FailedTripDebuggerID({:?})", self._raw_id)
    }
}
impl ::std::hash::Hash for FailedTripDebuggerID {
    fn hash<H: ::std::hash::Hasher>(&self, state: &mut H) {
        self._raw_id.hash(state);
    }
}
impl PartialEq for FailedTripDebuggerID {
    fn eq(&self, other: &FailedTripDebuggerID) -> bool {
        self._raw_id == other._raw_id
    }
}
impl Eq for FailedTripDebuggerID {}

impl TypedID for FailedTripDebuggerID {
    type Target = FailedTripDebugger;

    fn from_raw(id: RawID) -> Self {
        FailedTripDebuggerID { _raw_id: id }
    }

    fn as_raw(&self) -> RawID {
        self._raw_id
    }
}

impl FailedTripDebuggerID {
    pub fn spawn(rough_source: RoughLocationID, rough_destination: RoughLocationID, world: &mut World) -> Self {
        let id = FailedTripDebuggerID::from_raw(world.allocate_instance_id::<FailedTripDebugger>());
        let swarm = world.local_broadcast::<FailedTripDebugger>();
        world.send(swarm, MSG_FailedTripDebugger_spawn(id, rough_source, rough_destination));
        id
    }
    
    pub fn done(self, world: &mut World) {
        world.send(self.as_raw(), MSG_FailedTripDebugger_done());
    }
}

#[derive(Compact, Clone)] #[allow(non_camel_case_types)]
struct MSG_FailedTripDebugger_spawn(pub FailedTripDebuggerID, pub RoughLocationID, pub RoughLocationID);
#[derive(Copy, Clone)] #[allow(non_camel_case_types)]
struct MSG_FailedTripDebugger_done();

impl Into<PositionRequesterID> for FailedTripDebuggerID {
    fn into(self) -> PositionRequesterID {
        PositionRequesterID::from_raw(self.as_raw())
    }
}

#[allow(unused_variables)]
#[allow(unused_mut)]
pub fn auto_setup(system: &mut ActorSystem) {
    TripListenerID::register_trait(system);
    LocationRequesterID::register_implementor::<Trip>(system);
    system.add_spawner::<Trip, _, _>(
        |&MSG_Trip_spawn(id, rough_source, rough_destination, listener, instant), world| {
            Trip::spawn(id, rough_source, rough_destination, listener, instant, world)
        }, false
    );
    
    system.add_handler::<Trip, _, _>(
        |&MSG_Trip_finish(result), instance, world| {
            instance.finish(result, world)
        }, false
    );
    SleeperID::register_implementor::<TripCreator>(system);
    system.add_spawner::<TripCreator, _, _>(
        |&MSG_TripCreator_spawn(id, time), world| {
            TripCreator::spawn(id, time, world)
        }, false
    );
    
    system.add_handler::<TripCreator, _, _>(
        |&MSG_TripCreator_add_lane_for_trip(lane_id), instance, world| {
            instance.add_lane_for_trip(lane_id, world); Fate::Live
        }, false
    );
    
    system.add_handler::<Lane, _, _>(
        |&MSG_Lane_manually_spawn_car_add_lane(), instance, world| {
            instance.manually_spawn_car_add_lane(world); Fate::Live
        }, false
    );
    PositionRequesterID::register_implementor::<FailedTripDebugger>(system);
    system.add_spawner::<FailedTripDebugger, _, _>(
        |&MSG_FailedTripDebugger_spawn(id, rough_source, rough_destination), world| {
            FailedTripDebugger::spawn(id, rough_source, rough_destination, world)
        }, false
    );
    
    system.add_handler::<FailedTripDebugger, _, _>(
        |&MSG_FailedTripDebugger_done(), instance, world| {
            instance.done(world)
        }, false
    );
}