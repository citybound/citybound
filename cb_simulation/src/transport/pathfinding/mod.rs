use compact::{CDict, CVec, CHashMap};
use kay::{ActorSystem, Actor, World, TypedID};
use descartes::{P2};
use cb_time::units::Instant;

pub mod trip;
pub mod road_pathfinding;

const LOG_T: &str = "Pathfinding";

use cb_util::log::{debug};

#[derive(Copy, Clone)]
pub enum NetworkFlavor {
    CarNetwork,
    PedestrianNetwork,
}

pub trait Link: Actor {
    fn core(&self) -> &PathfindingCore;
    fn core_mut(&mut self) -> &mut PathfindingCore;

    fn self_as_route(&self) -> Option<(Location, CommunicatedRoutingEntry)>;
    fn can_be_landmark(&self) -> bool;

    fn map_connected_link_to_idx(&self, link: LinkID) -> Option<usize>;
    // TODO: would be nice to return impl Iterator here, but not supported yet in Traits
    fn successors(&self) -> Vec<LinkConnection>;
    fn predecessors(&self) -> Vec<LinkConnection>;

    fn after_route_forgotten(&mut self, forgotten_route: Location, world: &mut World);

    fn on_connect(&mut self) {
        self.core_mut().routing_timeout = ROUTING_TIMEOUT_AFTER_CHANGE;
    }

    fn on_disconnect(&mut self) {
        self.core_mut().routes = CHashMap::new();
        self.core_mut().routes_changed = true;
        self.core_mut().query_routes_next_tick = true;
    }

    fn pathfinding_tick(&mut self, world: &mut World) {
        if let Some(location) = self.core().location {
            for LinkConnection {
                link: successor, ..
            } in self.successors()
            {
                successor.join_landmark(
                    self.id_as(),
                    Location {
                        landmark: location.landmark,
                        link: successor,
                    },
                    self.core().hops_from_landmark + 1,
                    world,
                );
            }
        } else if self.can_be_landmark() && self.predecessors().len() >= MIN_LANDMARK_INCOMING {
            *self.core_mut() = PathfindingCore {
                location: Some(Location::landmark(self.id_as())),
                hops_from_landmark: 0,
                learned_landmark_from: Some(self.id_as()),
                routes: CHashMap::new(),
                routes_changed: true,
                query_routes_next_tick: false,
                tell_to_forget_next_tick: CVec::new(),
                routing_timeout: ROUTING_TIMEOUT_AFTER_CHANGE,
                attachees: self.core().attachees.clone(),
            }
        }

        if self.core().routing_timeout > 0 {
            self.core_mut().routing_timeout -= 1;
        } else {
            if self.core().query_routes_next_tick {
                for successor in self.successors() {
                    successor
                        .link
                        .query_routes(self.id_as(), successor.connection_cost, world);
                }
                self.core_mut().query_routes_next_tick = false;
            }

            if !self.core().tell_to_forget_next_tick.is_empty() {
                for predecessor in self.predecessors() {
                    predecessor.link.forget_routes(
                        self.core().tell_to_forget_next_tick.clone(),
                        self.id_as(),
                        world,
                    );
                }
                self.core_mut().tell_to_forget_next_tick.clear();
            }

            if self.core().routes_changed {
                for predecessor in self.predecessors() {
                    self.query_routes(predecessor.link, predecessor.connection_cost, world);
                }
                self.core_mut().routes_changed = false;
            }
        }
    }

    fn query_routes(&mut self, requester: LinkID, connection_cost: f32, world: &mut World) {
        requester.on_routes(
            self.core()
                .routes
                .pairs()
                .map(|(&destination, &stored_entry)| {
                    (
                        destination,
                        CommunicatedRoutingEntry {
                            distance: stored_entry.distance + connection_cost,
                            distance_hops: stored_entry.distance_hops + 1,
                        },
                    )
                })
                .chain(self.self_as_route())
                .collect(),
            self.id_as(),
            world,
        );
    }

    fn on_routes(
        &mut self,
        new_routes: &CDict<Location, CommunicatedRoutingEntry>,
        from: LinkID,
        world: &mut World,
    ) {
        if let Some(from_connection_idx) = self.map_connected_link_to_idx(from) {
            for (
                &destination,
                &CommunicatedRoutingEntry {
                    distance: new_distance,
                    distance_hops: new_distance_hops,
                },
            ) in new_routes.pairs()
            {
                if destination.is_landmark()
                    || new_distance_hops <= IDEAL_LANDMARK_RADIUS
                    || self
                        .core()
                        .location
                        .map(|self_dest| self_dest.landmark == destination.landmark)
                        .unwrap_or(false)
                {
                    let insert = self
                        .core()
                        .routes
                        .get(destination)
                        .map(|&StoredRoutingEntry { distance, .. }| new_distance < distance)
                        .unwrap_or(true);
                    if insert {
                        self.core_mut().routes.insert(
                            destination,
                            StoredRoutingEntry {
                                distance: new_distance,
                                distance_hops: new_distance_hops,
                                outgoing_idx: from_connection_idx as u8,
                                learned_from: from,
                            },
                        );
                        self.core_mut().routes_changed = true;
                    }
                }
            }
        } else {
            debug(
                LOG_T,
                format!("{:?} not yet connected to {:?}", self.id(), from),
                self.id(),
                world,
            );
        }
    }

    fn forget_routes(&mut self, forget: &CVec<Location>, from: LinkID, world: &mut World) {
        let mut forgotten_routes = CVec::<Location>::new();
        for &destination_to_forget in forget.iter() {
            let forget = if let Some(routing_info) = self.core().routes.get(destination_to_forget) {
                routing_info.learned_from == from
            } else {
                false
            };
            if forget {
                self.core_mut().routes.remove(destination_to_forget);
                self.after_route_forgotten(destination_to_forget, world);
                forgotten_routes.push(destination_to_forget);
            }
        }
        self.core_mut().tell_to_forget_next_tick = forgotten_routes;
    }

    fn join_landmark(
        &mut self,
        from: LinkID,
        join_as: Location,
        hops_from_landmark: u8,
        world: &mut World,
    ) {
        let join = self
            .core()
            .location
            .map(|self_location| {
                join_as != self_location
                    && (if self_location.is_landmark() {
                        hops_from_landmark < IDEAL_LANDMARK_RADIUS
                            && join_as.landmark.as_raw().instance_id
                                < self.id().as_raw().instance_id
                    } else {
                        hops_from_landmark < self.core().hops_from_landmark
                            || self
                                .core()
                                .learned_landmark_from
                                .map(|learned_from| learned_from == from)
                                .unwrap_or(false)
                    })
            })
            .unwrap_or(true);
        if join {
            let tell_to_forget_next_tick = self
                .core()
                .routes
                .keys()
                .cloned()
                .chain(self.core().location.into_iter())
                .collect();

            for attachee in &self.core().attachees {
                attachee.location_changed(self.core().location, Some(join_as), world);
            }

            *self.core_mut() = PathfindingCore {
                location: Some(join_as),
                learned_landmark_from: Some(from),
                hops_from_landmark,
                routes: CHashMap::new(),
                routes_changed: true,
                query_routes_next_tick: true,
                tell_to_forget_next_tick,
                routing_timeout: ROUTING_TIMEOUT_AFTER_CHANGE,
                attachees: self.core().attachees.clone(),
            };
        }
    }

    fn get_distance_to(
        &mut self,
        destination: Location,
        requester: DistanceRequesterID,
        world: &mut World,
    ) {
        let maybe_distance = self
            .core()
            .routes
            .get(destination)
            .or_else(|| self.core().routes.get(destination.landmark_destination()))
            .map(|routing_info| routing_info.distance);
        requester.on_distance(maybe_distance, world);
    }

    fn add_attachee(&mut self, attachee: AttacheeID, _: &mut World) {
        self.core_mut().attachees.push(attachee);
    }

    fn remove_attachee(&mut self, attachee: AttacheeID, _: &mut World) {
        self.core_mut().attachees.retain(|a| *a != attachee);
    }
}

#[derive(Copy, Clone)]
pub struct LinkConnection {
    link: LinkID,
    connection_cost: f32,
}

#[derive(Compact, Clone, Default)]
pub struct PathfindingCore {
    pub network_flavor: NetworkFlavor,
    pub location: Option<Location>,
    pub hops_from_landmark: u8,
    pub learned_landmark_from: Option<LinkID>,
    pub routes: CHashMap<Location, StoredRoutingEntry>,
    pub routes_changed: bool,
    pub tell_to_forget_next_tick: CVec<Location>,
    pub query_routes_next_tick: bool,
    pub routing_timeout: u16,
    attachees: CVec<AttacheeID>,
}

#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub struct Location {
    pub landmark: LinkID,
    pub link: LinkID,
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
    fn landmark(landmark: LinkID) -> Self {
        Location {
            landmark,
            link: landmark,
        }
    }
    pub fn is_landmark(&self) -> bool {
        self.landmark == self.link
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
        network_flavor: NetworkFlavor,
        world: &mut World,
    );
}

#[derive(Copy, Clone)]
pub struct StoredRoutingEntry {
    pub outgoing_idx: u8,
    pub distance: f32,
    distance_hops: u8,
    learned_from: LinkID,
}

#[derive(Copy, Clone)]
pub struct CommunicatedRoutingEntry {
    pub distance: f32,
    pub distance_hops: u8,
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

use cb_time::actors::TimeID;

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
