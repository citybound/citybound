//! This is all auto-generated. Do not touch.
#![rustfmt::skip]
#[allow(unused_imports)]
use kay::{ActorSystem, TypedID, RawID, Fate, Actor, TraitIDFrom, ActorOrActorTrait};
#[allow(unused_imports)]
use super::*;

#[derive(Serialize, Deserialize)] #[serde(transparent)]
pub struct ObstacleContainerID {
    _raw_id: RawID
}

impl Copy for ObstacleContainerID {}
impl Clone for ObstacleContainerID { fn clone(&self) -> Self { *self } }
impl ::std::fmt::Debug for ObstacleContainerID {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        write!(f, "ObstacleContainerID({:?})", self._raw_id)
    }
}
impl ::std::hash::Hash for ObstacleContainerID {
    fn hash<H: ::std::hash::Hasher>(&self, state: &mut H) {
        self._raw_id.hash(state);
    }
}
impl PartialEq for ObstacleContainerID {
    fn eq(&self, other: &ObstacleContainerID) -> bool {
        self._raw_id == other._raw_id
    }
}
impl Eq for ObstacleContainerID {}

pub struct ObstacleContainerRepresentative;

impl ActorOrActorTrait for ObstacleContainerRepresentative {
    type ID = ObstacleContainerID;
}

impl TypedID for ObstacleContainerID {
    type Target = ObstacleContainerRepresentative;

    fn from_raw(id: RawID) -> Self {
        ObstacleContainerID { _raw_id: id }
    }

    fn as_raw(&self) -> RawID {
        self._raw_id
    }
}

impl<Act: Actor + ObstacleContainer> TraitIDFrom<Act> for ObstacleContainerID {}

impl ObstacleContainerID {
    pub fn start_connecting_overlaps(self, other: CVec < ObstacleContainerID >, world: &mut World) {
        world.send(self.as_raw(), MSG_ObstacleContainer_start_connecting_overlaps(other));
    }
    
    pub fn connect_overlaps(self, other_obstacle_container: ObstacleContainerID, other_path: LinePath, other_width: N, reply_needed: bool, world: &mut World) {
        world.send(self.as_raw(), MSG_ObstacleContainer_connect_overlaps(other_obstacle_container, other_path, other_width, reply_needed));
    }
    
    pub fn add_obstacles(self, obstacles: CVec < Obstacle >, from: ObstacleContainerID, world: &mut World) {
        world.send(self.as_raw(), MSG_ObstacleContainer_add_obstacles(obstacles, from));
    }
    
    pub fn disconnect(self, other: ObstacleContainerID, world: &mut World) {
        world.send(self.as_raw(), MSG_ObstacleContainer_disconnect(other));
    }
    
    pub fn on_confirm_disconnect(self, world: &mut World) {
        world.send(self.as_raw(), MSG_ObstacleContainer_on_confirm_disconnect());
    }

    pub fn register_trait(system: &mut ActorSystem) {
        system.register_trait::<ObstacleContainerRepresentative>();
        system.register_trait_message::<MSG_ObstacleContainer_start_connecting_overlaps>();
        system.register_trait_message::<MSG_ObstacleContainer_connect_overlaps>();
        system.register_trait_message::<MSG_ObstacleContainer_add_obstacles>();
        system.register_trait_message::<MSG_ObstacleContainer_disconnect>();
        system.register_trait_message::<MSG_ObstacleContainer_on_confirm_disconnect>();
    }

    pub fn register_implementor<Act: Actor + ObstacleContainer>(system: &mut ActorSystem) {
        system.register_implementor::<Act, ObstacleContainerRepresentative>();
        system.add_handler::<Act, _, _>(
            |&MSG_ObstacleContainer_start_connecting_overlaps(ref other), instance, world| {
                instance.start_connecting_overlaps(other, world); Fate::Live
            }, false
        );
        
        system.add_handler::<Act, _, _>(
            |&MSG_ObstacleContainer_connect_overlaps(other_obstacle_container, ref other_path, other_width, reply_needed), instance, world| {
                instance.connect_overlaps(other_obstacle_container, other_path, other_width, reply_needed, world); Fate::Live
            }, false
        );
        
        system.add_handler::<Act, _, _>(
            |&MSG_ObstacleContainer_add_obstacles(ref obstacles, from), instance, world| {
                instance.add_obstacles(obstacles, from, world); Fate::Live
            }, false
        );
        
        system.add_handler::<Act, _, _>(
            |&MSG_ObstacleContainer_disconnect(other), instance, world| {
                instance.disconnect(other, world); Fate::Live
            }, false
        );
        
        system.add_handler::<Act, _, _>(
            |&MSG_ObstacleContainer_on_confirm_disconnect(), instance, world| {
                instance.on_confirm_disconnect(world)
            }, false
        );
    }
}

#[derive(Compact, Clone)] #[allow(non_camel_case_types)]
struct MSG_ObstacleContainer_start_connecting_overlaps(pub CVec < ObstacleContainerID >);
#[derive(Compact, Clone)] #[allow(non_camel_case_types)]
struct MSG_ObstacleContainer_connect_overlaps(pub ObstacleContainerID, pub LinePath, pub N, pub bool);
#[derive(Compact, Clone)] #[allow(non_camel_case_types)]
struct MSG_ObstacleContainer_add_obstacles(pub CVec < Obstacle >, pub ObstacleContainerID);
#[derive(Compact, Clone)] #[allow(non_camel_case_types)]
struct MSG_ObstacleContainer_disconnect(pub ObstacleContainerID);
#[derive(Copy, Clone)] #[allow(non_camel_case_types)]
struct MSG_ObstacleContainer_on_confirm_disconnect();
#[derive(Serialize, Deserialize)] #[serde(transparent)]
pub struct CarLaneLikeID {
    _raw_id: RawID
}

impl Copy for CarLaneLikeID {}
impl Clone for CarLaneLikeID { fn clone(&self) -> Self { *self } }
impl ::std::fmt::Debug for CarLaneLikeID {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        write!(f, "CarLaneLikeID({:?})", self._raw_id)
    }
}
impl ::std::hash::Hash for CarLaneLikeID {
    fn hash<H: ::std::hash::Hasher>(&self, state: &mut H) {
        self._raw_id.hash(state);
    }
}
impl PartialEq for CarLaneLikeID {
    fn eq(&self, other: &CarLaneLikeID) -> bool {
        self._raw_id == other._raw_id
    }
}
impl Eq for CarLaneLikeID {}

pub struct CarLaneLikeRepresentative;

impl ActorOrActorTrait for CarLaneLikeRepresentative {
    type ID = CarLaneLikeID;
}

impl TypedID for CarLaneLikeID {
    type Target = CarLaneLikeRepresentative;

    fn from_raw(id: RawID) -> Self {
        CarLaneLikeID { _raw_id: id }
    }

    fn as_raw(&self) -> RawID {
        self._raw_id
    }
}

impl<Act: Actor + CarLaneLike> TraitIDFrom<Act> for CarLaneLikeID {}

impl CarLaneLikeID {
    pub fn add_car(self, car: LaneCar, from: Option < CarLaneLikeID >, instant: Instant, world: &mut World) {
        world.send(self.as_raw(), MSG_CarLaneLike_add_car(car, from, instant));
    }

    pub fn register_trait(system: &mut ActorSystem) {
        system.register_trait::<CarLaneLikeRepresentative>();
        system.register_trait_message::<MSG_CarLaneLike_add_car>();
    }

    pub fn register_implementor<Act: Actor + CarLaneLike>(system: &mut ActorSystem) {
        system.register_implementor::<Act, CarLaneLikeRepresentative>();
        system.add_handler::<Act, _, _>(
            |&MSG_CarLaneLike_add_car(car, from, instant), instance, world| {
                instance.add_car(car, from, instant, world); Fate::Live
            }, false
        );
    }
}

#[derive(Compact, Clone)] #[allow(non_camel_case_types)]
struct MSG_CarLaneLike_add_car(pub LaneCar, pub Option < CarLaneLikeID >, pub Instant);



impl CarLaneID {
    pub fn on_signal_changed(self, from: CarLaneID, new_green: bool, world: &mut World) {
        world.send(self.as_raw(), MSG_CarLane_on_signal_changed(from, new_green));
    }
}

#[derive(Compact, Clone)] #[allow(non_camel_case_types)]
struct MSG_CarLane_on_signal_changed(pub CarLaneID, pub bool);

impl Into<CarLaneLikeID> for CarLaneID {
    fn into(self) -> CarLaneLikeID {
        CarLaneLikeID::from_raw(self.as_raw())
    }
}

impl Into<ObstacleContainerID> for CarLaneID {
    fn into(self) -> ObstacleContainerID {
        ObstacleContainerID::from_raw(self.as_raw())
    }
}

impl Into<TemporalID> for CarLaneID {
    fn into(self) -> TemporalID {
        TemporalID::from_raw(self.as_raw())
    }
}


impl CarSwitchLaneID {
    
}



impl Into<CarLaneLikeID> for CarSwitchLaneID {
    fn into(self) -> CarLaneLikeID {
        CarLaneLikeID::from_raw(self.as_raw())
    }
}

impl Into<ObstacleContainerID> for CarSwitchLaneID {
    fn into(self) -> ObstacleContainerID {
        ObstacleContainerID::from_raw(self.as_raw())
    }
}

impl Into<TemporalID> for CarSwitchLaneID {
    fn into(self) -> TemporalID {
        TemporalID::from_raw(self.as_raw())
    }
}


impl SidewalkID {
    pub fn add_pedestrian(self, pedestrian: Pedestrian, instant: Instant, world: &mut World) {
        world.send(self.as_raw(), MSG_Sidewalk_add_pedestrian(pedestrian, instant));
    }
}

#[derive(Compact, Clone)] #[allow(non_camel_case_types)]
struct MSG_Sidewalk_add_pedestrian(pub Pedestrian, pub Instant);

impl Into<ObstacleContainerID> for SidewalkID {
    fn into(self) -> ObstacleContainerID {
        ObstacleContainerID::from_raw(self.as_raw())
    }
}

impl Into<TemporalID> for SidewalkID {
    fn into(self) -> TemporalID {
        TemporalID::from_raw(self.as_raw())
    }
}

#[allow(unused_variables)]
#[allow(unused_mut)]
pub fn auto_setup(system: &mut ActorSystem) {
    ObstacleContainerID::register_trait(system);
    CarLaneLikeID::register_trait(system);
    CarLaneLikeID::register_implementor::<CarLane>(system);
    ObstacleContainerID::register_implementor::<CarLane>(system);
    TemporalID::register_implementor::<CarLane>(system);
    system.add_handler::<CarLane, _, _>(
        |&MSG_CarLane_on_signal_changed(from, new_green), instance, world| {
            instance.on_signal_changed(from, new_green, world); Fate::Live
        }, false
    );
    CarLaneLikeID::register_implementor::<CarSwitchLane>(system);
    ObstacleContainerID::register_implementor::<CarSwitchLane>(system);
    TemporalID::register_implementor::<CarSwitchLane>(system);
    
    ObstacleContainerID::register_implementor::<Sidewalk>(system);
    TemporalID::register_implementor::<Sidewalk>(system);
    system.add_handler::<Sidewalk, _, _>(
        |&MSG_Sidewalk_add_pedestrian(pedestrian, instant), instance, world| {
            instance.add_pedestrian(pedestrian, instant, world); Fate::Live
        }, false
    );
}