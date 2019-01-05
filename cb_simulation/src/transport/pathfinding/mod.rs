use compact::{CDict, CVec, CHashMap};
use kay::{ActorSystem, World};
use descartes::{P2};
use time::Instant;

pub mod trip;
pub mod road_pathfinding;

pub trait Node {
    fn core(&self) -> &PathfindingCore;
    fn core_mut(&mut self) -> &mut PathfindingCore;

    fn update_routes(&mut self, world: &mut World);
    fn query_routes(
        &mut self,
        requester: NodeID,
        custom_connection_cost: Option<f32>,
        world: &mut World,
    );
    fn on_routes(
        &mut self,
        new_routes: &CDict<Location, (f32, u8)>,
        from: NodeID,
        world: &mut World,
    );
    fn forget_routes(&mut self, forget: &CVec<Location>, from: NodeID, world: &mut World);
    fn join_landmark(
        &mut self,
        from: NodeID,
        join_as: Location,
        hops_from_landmark: u8,
        world: &mut World,
    );
    fn get_distance_to(
        &mut self,
        location: Location,
        requester: DistanceRequesterID,
        world: &mut World,
    );
    fn add_attachee(&mut self, attachee: AttacheeID, world: &mut World);
    fn remove_attachee(&mut self, attachee: AttacheeID, world: &mut World);
}

#[derive(Compact, Clone, Default)]
pub struct PathfindingCore {
    pub location: Option<Location>,
    pub hops_from_landmark: u8,
    pub learned_landmark_from: Option<NodeID>,
    pub routes: CHashMap<Location, RoutingInfo>,
    pub routes_changed: bool,
    pub tell_to_forget_next_tick: CVec<Location>,
    pub query_routes_next_tick: bool,
    pub routing_timeout: u16,
    attachees: CVec<AttacheeID>,
}

#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub struct Location {
    pub landmark: NodeID,
    pub node: NodeID,
}

#[derive(Copy, Clone, PartialEq, Debug)]
pub struct PreciseLocation {
    pub location: Location,
    pub offset: f32,
}

impl ::std::ops::Deref for PreciseLocation {
    type Target = Location;

    fn deref(&self) -> &Location {
        &self.location
    }
}

impl ::std::ops::DerefMut for PreciseLocation {
    fn deref_mut(&mut self) -> &mut Location {
        &mut self.location
    }
}

impl Location {
    fn landmark(landmark: NodeID) -> Self {
        Location {
            landmark,
            node: landmark,
        }
    }
    pub fn is_landmark(&self) -> bool {
        self.landmark == self.node
    }
    pub fn landmark_destination(&self) -> Self {
        Self::landmark(self.landmark)
    }
}

pub trait Attachee {
    fn location_changed(
        &mut self,
        _old: Option<Location>,
        new: Option<Location>,
        world: &mut World,
    );
}

#[derive(Copy, Clone)]
pub struct RoutingInfo {
    pub outgoing_idx: u8,
    pub distance: f32,
    distance_hops: u8,
    learned_from: NodeID,
    fresh: bool,
}

const IDEAL_LANDMARK_RADIUS: u8 = 3;
const MIN_LANDMARK_INCOMING: usize = 3;
const ROUTING_TIMEOUT_AFTER_CHANGE: u16 = 15;

pub enum RoughLocationResolve {
    Done(Option<PreciseLocation>, P2),
    SameAs(RoughLocationID),
}

pub trait RoughLocation {
    fn resolve(&self) -> RoughLocationResolve;

    fn resolve_as_location(
        &mut self,
        requester: LocationRequesterID,
        rough_location: RoughLocationID,
        instant: Instant,
        world: &mut World,
    ) {
        match self.resolve() {
            RoughLocationResolve::Done(maybe_location, _) => {
                requester.location_resolved(rough_location, maybe_location, instant, world);
            }
            RoughLocationResolve::SameAs(other_rough_location) => {
                other_rough_location.resolve_as_location(requester, rough_location, instant, world);
            }
        }
    }

    fn resolve_as_position(
        &mut self,
        requester: PositionRequesterID,
        rough_location: RoughLocationID,
        world: &mut World,
    ) {
        match self.resolve() {
            RoughLocationResolve::Done(_, position) => {
                requester.position_resolved(rough_location, position, world);
            }
            RoughLocationResolve::SameAs(other_rough_location) => {
                other_rough_location.resolve_as_position(requester, rough_location, world);
            }
        }
    }
}

pub trait LocationRequester {
    fn location_resolved(
        &mut self,
        rough_location: RoughLocationID,
        location: Option<PreciseLocation>,
        instant: Instant,
        world: &mut World,
    );
}

pub trait PositionRequester {
    fn position_resolved(
        &mut self,
        rough_location: RoughLocationID,
        position: P2,
        world: &mut World,
    );
}

pub trait DistanceRequester {
    fn on_distance(&mut self, maybe_distance: Option<f32>, world: &mut World);
}

use time::TimeID;

pub fn setup(system: &mut ActorSystem) {
    trip::setup(system);
    road_pathfinding::auto_setup(system);
    auto_setup(system);
}

pub fn spawn(world: &mut World, time: TimeID) {
    trip::spawn(world, time);
}

mod kay_auto;
pub use self::kay_auto::*;
