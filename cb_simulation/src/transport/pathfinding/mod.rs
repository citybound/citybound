use compact::{CDict, CVec, CHashMap};
use kay::{ActorSystem, World, TypedID, Actor};
use descartes::{P2};
use super::lane::{Lane, LaneID, SwitchLane, SwitchLaneID};
use super::lane::connectivity::{Interaction, InteractionKind, OverlapKind};
use time::Instant;

// TODO: MAKE TRANSFER LANE NOT PARTICIPATE AT ALL IN PATHFINDING -> MUCH SIMPLER

pub mod trip;
use self::trip::{TripResult, TripFate};

use log::{debug};
const LOG_T: &str = "Pathfinding";

pub trait Node {
    fn update_routes(&mut self, world: &mut World);
    fn query_routes(&mut self, requester: NodeID, is_switch: bool, world: &mut World);
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
pub struct PathfindingInfo {
    pub location: Option<Location>,
    pub hops_from_landmark: u8,
    pub learned_landmark_from: Option<NodeID>,
    pub routes: CHashMap<Location, RoutingInfo>,
    pub routes_changed: bool,
    pub tell_to_forget_next_tick: CVec<Location>,
    pub query_routes_next_tick: bool,
    pub routing_timeout: u16,
    attachees: CVec<AttacheeID>,
    pub debug_highlight_for: CHashMap<LaneID, ()>,
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

pub fn on_connect(lane: &mut Lane) {
    lane.pathfinding.routing_timeout = ROUTING_TIMEOUT_AFTER_CHANGE;
}

use super::microtraffic::LaneLikeID;

pub fn on_disconnect(lane: &mut Lane, disconnected_id: LaneLikeID) {
    // TODO: ugly: untyped RawID shenanigans
    let new_routes = lane
        .pathfinding
        .routes
        .pairs()
        .filter_map(|(destination, route)| {
            if route.learned_from.as_raw() == disconnected_id.as_raw() {
                None
            } else {
                Some((*destination, *route))
            }
        })
        .collect();
    lane.pathfinding.routes = new_routes;
    lane.pathfinding.routes_changed = true;
    lane.pathfinding.query_routes_next_tick = true;
}

const IDEAL_LANDMARK_RADIUS: u8 = 3;
const MIN_LANDMARK_INCOMING: usize = 3;
const ROUTING_TIMEOUT_AFTER_CHANGE: u16 = 15;
const LANE_CHANGE_COST_LEFT: f32 = 5.0;
const LANE_CHANGE_COST_RIGHT: f32 = 3.0;

impl Node for Lane {
    fn update_routes(&mut self, world: &mut World) {
        if let Some(location) = self.pathfinding.location {
            for successor in successors(self) {
                successor.join_landmark(
                    self.id_as(),
                    Location {
                        landmark: location.landmark,
                        node: successor,
                    },
                    self.pathfinding.hops_from_landmark + 1,
                    world,
                );
            }
        } else if !self.connectivity.on_intersection
            && predecessors(self).count() >= MIN_LANDMARK_INCOMING
        {
            self.pathfinding = PathfindingInfo {
                location: Some(Location::landmark(self.id_as())),
                hops_from_landmark: 0,
                learned_landmark_from: Some(self.id_as()),
                routes: CHashMap::new(),
                routes_changed: true,
                query_routes_next_tick: false,
                tell_to_forget_next_tick: CVec::new(),
                routing_timeout: ROUTING_TIMEOUT_AFTER_CHANGE,
                attachees: self.pathfinding.attachees.clone(),
                debug_highlight_for: self.pathfinding.debug_highlight_for.clone(),
            }
        }

        if self.pathfinding.routing_timeout > 0 {
            self.pathfinding.routing_timeout -= 1;
        } else {
            if self.pathfinding.query_routes_next_tick {
                for successor in successors(self) {
                    successor.query_routes(self.id_as(), false, world);
                }
                self.pathfinding.query_routes_next_tick = false;
            }

            if !self.pathfinding.tell_to_forget_next_tick.is_empty() {
                for (_, predecessor, _) in predecessors(self) {
                    predecessor.forget_routes(
                        self.pathfinding.tell_to_forget_next_tick.clone(),
                        self.id_as(),
                        world,
                    );
                }
                self.pathfinding.tell_to_forget_next_tick.clear();
            }

            if self.pathfinding.routes_changed {
                for (_, predecessor, is_switch) in predecessors(self) {
                    let self_cost = if is_switch {
                        0.0
                    } else {
                        self.construction.length
                    };
                    predecessor.on_routes(
                        self.pathfinding
                            .routes
                            .pairs()
                            .filter_map(
                                |(
                                    &destination,
                                    &RoutingInfo {
                                        distance,
                                        distance_hops,
                                        ..
                                    },
                                )| {
                                    if true
                                    // fresh
                                    {
                                        Some((
                                            destination,
                                            (distance + self_cost, distance_hops + 1),
                                        ))
                                    } else {
                                        None
                                    }
                                },
                            )
                            .chain(if self.connectivity.on_intersection {
                                None
                            } else {
                                self.pathfinding
                                    .location
                                    .map(|destination| (destination, (self_cost, 0)))
                            })
                            .collect(),
                        self.id_as(),
                        world,
                    );
                }
                for routing_info in self.pathfinding.routes.values_mut() {
                    routing_info.fresh = false;
                }
                self.pathfinding.routes_changed = false;
            }
        }
    }

    fn query_routes(&mut self, requester: NodeID, is_switch: bool, world: &mut World) {
        let self_cost = if is_switch {
            0.0
        } else {
            self.construction.length
        };
        requester.on_routes(
            self.pathfinding
                .routes
                .pairs()
                .map(
                    |(
                        &destination,
                        &RoutingInfo {
                            distance,
                            distance_hops,
                            ..
                        },
                    )| {
                        (destination, (distance + self_cost, distance_hops + 1))
                    },
                )
                .chain(if self.connectivity.on_intersection {
                    None
                } else {
                    self.pathfinding
                        .location
                        .map(|destination| (destination, (self_cost, 0)))
                })
                .collect(),
            self.id_as(),
            world,
        );
    }

    fn on_routes(
        &mut self,
        new_routes: &CDict<Location, (f32, u8)>,
        from: NodeID,
        world: &mut World,
    ) {
        if let Some(from_interaction_idx) =
            self.connectivity
                .interactions
                .iter()
                .position(|interaction| {
                    // TODO: ugly: untyped RawID shenanigans
                    interaction.partner_lane.as_raw() == from.as_raw()
                }) {
            for (&destination, &(new_distance, new_distance_hops)) in new_routes.pairs() {
                if destination.is_landmark() || new_distance_hops <= IDEAL_LANDMARK_RADIUS || self
                    .pathfinding
                    .location
                    .map(|self_dest| self_dest.landmark == destination.landmark)
                    .unwrap_or(false)
                {
                    let insert = self
                        .pathfinding
                        .routes
                        .get(destination)
                        .map(|&RoutingInfo { distance, .. }| new_distance < distance)
                        .unwrap_or(true);
                    if insert {
                        self.pathfinding.routes.insert(
                            destination,
                            RoutingInfo {
                                distance: new_distance,
                                distance_hops: new_distance_hops,
                                outgoing_idx: from_interaction_idx as u8,
                                learned_from: from,
                                fresh: true,
                            },
                        );
                        self.pathfinding.routes_changed = true;
                    }
                }
            }
        } else {
            debug(
                LOG_T,
                format!(
                    "{:?} not yet connected to {:?}",
                    self.id.as_raw(),
                    from.as_raw()
                ),
                self.id(),
                world,
            );
        }
    }

    fn forget_routes(&mut self, forget: &CVec<Location>, from: NodeID, world: &mut World) {
        let mut forgotten = CVec::<Location>::new();
        for destination_to_forget in forget.iter() {
            let forget =
                if let Some(routing_info) = self.pathfinding.routes.get(*destination_to_forget) {
                    routing_info.learned_from == from
                } else {
                    false
                };
            if forget {
                self.pathfinding.routes.remove(*destination_to_forget);
                let self_as_rough_location = self.id_as();
                if destination_to_forget.is_landmark() {
                    self.microtraffic.cars.retain(|car| {
                        if car.destination.landmark == destination_to_forget.landmark {
                            car.trip.finish(
                                TripResult {
                                    location_now: Some(self_as_rough_location),
                                    fate: TripFate::RouteForgotten,
                                },
                                world,
                            );
                            false
                        } else {
                            true
                        }
                    })
                } else {
                    self.microtraffic.cars.retain(|car| {
                        if &car.destination.location == destination_to_forget {
                            car.trip.finish(
                                TripResult {
                                    location_now: Some(self_as_rough_location),
                                    fate: TripFate::RouteForgotten,
                                },
                                world,
                            );
                            false
                        } else {
                            true
                        }
                    })
                }
                forgotten.push(*destination_to_forget);
            }
        }
        self.pathfinding.tell_to_forget_next_tick = forgotten;
    }

    fn join_landmark(
        &mut self,
        from: NodeID,
        join_as: Location,
        hops_from_landmark: u8,
        world: &mut World,
    ) {
        let join = self
            .pathfinding
            .location
            .map(|self_location| {
                join_as != self_location
                    && (if self_location.is_landmark() {
                        hops_from_landmark < IDEAL_LANDMARK_RADIUS
                            && join_as.landmark.as_raw().instance_id < self.id.as_raw().instance_id
                    } else {
                        hops_from_landmark < self.pathfinding.hops_from_landmark || self
                            .pathfinding
                            .learned_landmark_from
                            .map(|learned_from| learned_from == from)
                            .unwrap_or(false)
                    })
            })
            .unwrap_or(true);
        if join {
            let tell_to_forget_next_tick = self
                .pathfinding
                .routes
                .keys()
                .cloned()
                .chain(self.pathfinding.location.into_iter())
                .collect();

            for attachee in &self.pathfinding.attachees {
                attachee.location_changed(self.pathfinding.location, Some(join_as), world);
            }

            self.pathfinding = PathfindingInfo {
                location: Some(join_as),
                learned_landmark_from: Some(from),
                hops_from_landmark,
                routes: CHashMap::new(),
                routes_changed: true,
                query_routes_next_tick: true,
                tell_to_forget_next_tick,
                routing_timeout: ROUTING_TIMEOUT_AFTER_CHANGE,
                attachees: self.pathfinding.attachees.clone(),
                debug_highlight_for: self.pathfinding.debug_highlight_for.clone(),
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
            .pathfinding
            .routes
            .get(destination)
            .or_else(|| {
                self.pathfinding
                    .routes
                    .get(destination.landmark_destination())
            })
            .map(|routing_info| routing_info.distance);
        requester.on_distance(maybe_distance, world);
    }

    fn add_attachee(&mut self, attachee: AttacheeID, _: &mut World) {
        self.pathfinding.attachees.push(attachee);
    }

    fn remove_attachee(&mut self, attachee: AttacheeID, _: &mut World) {
        self.pathfinding.attachees.retain(|a| *a != attachee);
    }
}

#[allow(clippy::needless_lifetimes)]
fn successors<'a>(lane: &'a Lane) -> impl Iterator<Item = NodeID> + 'a {
    // TODO: ugly: untyped RawID shenanigans
    lane.connectivity
        .interactions
        .iter()
        .filter_map(|interaction| match *interaction {
            Interaction {
                partner_lane,
                kind:
                    InteractionKind::Overlap {
                        kind: OverlapKind::Transfer,
                        ..
                    },
                ..
            }
            | Interaction {
                partner_lane,
                kind: InteractionKind::Next { .. },
                ..
            } => Some(NodeID::from_raw(partner_lane.as_raw())),
            _ => None,
        })
}

#[allow(clippy::needless_lifetimes)]
fn predecessors<'a>(lane: &'a Lane) -> impl Iterator<Item = (u8, NodeID, bool)> + 'a {
    lane.connectivity
        .interactions
        .iter()
        .enumerate()
        .filter_map(|(i, interaction)| match *interaction {
            // TODO: ugly: untyped RawID shenanigans
            Interaction {
                partner_lane,
                kind:
                    InteractionKind::Overlap {
                        kind: OverlapKind::Transfer,
                        ..
                    },
                ..
            } => Some((i as u8, NodeID::from_raw(partner_lane.as_raw()), true)),
            // TODO: ugly: untyped RawID shenanigans
            Interaction {
                partner_lane,
                kind: InteractionKind::Previous { .. },
                ..
            } => Some((i as u8, NodeID::from_raw(partner_lane.as_raw()), false)),
            _ => None,
        })
}

pub fn on_unbuild(lane: &Lane, world: &mut World) {
    for attachee in &lane.pathfinding.attachees {
        attachee.location_changed(lane.pathfinding.location, None, world);
    }
}

impl RoughLocation for Lane {
    fn resolve(&self) -> RoughLocationResolve {
        RoughLocationResolve::Done(
            self.pathfinding.location.map(|location| PreciseLocation {
                location,
                offset: 0.0,
            }),
            self.construction.path.along(self.construction.length / 2.0),
        )
    }
}

impl Node for SwitchLane {
    fn update_routes(&mut self, _: &mut World) {}

    fn query_routes(&mut self, requester: NodeID, _is_switch: bool, world: &mut World) {
        // TODO: ugly: untyped RawID shenanigans
        let requester_lane = LaneID::from_raw(requester.as_raw());
        if let Some(other_lane) = self.other_side(requester_lane) {
            let other_lane: NodeID = other_lane.into();
            other_lane.query_routes(self.id_as(), true, world);
        }
    }

    fn on_routes(
        &mut self,
        new_routes: &CDict<Location, (f32, u8)>,
        from: NodeID,
        world: &mut World,
    ) {
        let from_lane = LaneID::from_raw(from.as_raw());
        if let Some(other_lane) = self.other_side(from_lane) {
            let other_lane: NodeID = other_lane.into();
            other_lane.on_routes(
                new_routes
                    .pairs()
                    .map(|(&destination, &(distance, hops))| {
                        // TODO: ugly: untyped RawID shenanigans
                        let change_cost =
                            if from.as_raw()
                                == self.connectivity.left.expect("should have left").0.as_raw()
                            {
                                LANE_CHANGE_COST_RIGHT
                            } else {
                                LANE_CHANGE_COST_LEFT
                            };
                        (destination, (distance + change_cost, hops))
                    })
                    .collect(),
                self.id_as(),
                world,
            );
        }
    }

    fn forget_routes(&mut self, forget: &CVec<Location>, from: NodeID, world: &mut World) {
        let from_lane = LaneID::from_raw(from.as_raw());
        if let Some(other_lane) = self.other_side(from_lane) {
            let other_lane: NodeID = other_lane.into();
            other_lane.forget_routes(forget.clone(), self.id_as(), world);
        }
    }

    fn join_landmark(
        &mut self,
        from: NodeID,
        join_as: Location,
        hops_from_landmark: u8,
        world: &mut World,
    ) {
        let from_lane = LaneID::from_raw(from.as_raw());
        if let Some(other_lane) = self.other_side(from_lane) {
            let other_lane: NodeID = other_lane.into();
            other_lane.join_landmark(
                self.id_as(),
                Location {
                    landmark: join_as.landmark,
                    node: other_lane,
                },
                hops_from_landmark,
                world,
            );
        }
    }

    fn get_distance_to(
        &mut self,
        _location: Location,
        _requester: DistanceRequesterID,
        _: &mut World,
    ) {
        unimplemented!()
    }

    fn add_attachee(&mut self, _attachee: AttacheeID, _: &mut World) {}
    fn remove_attachee(&mut self, _attachee: AttacheeID, _: &mut World) {}
}

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

pub const DEBUG_VIEW_CONNECTIVITY: bool = false;

impl Lane {
    pub fn start_debug_connectivity(&self, world: &mut World) {
        for &Location { node, .. } in self.pathfinding.routes.keys() {
            // TODO: ugly: untyped RawID shenanigans
            if node.as_raw().local_broadcast() == LaneID::local_broadcast(world).as_raw() {
                let lane = LaneID::from_raw(node.as_raw());
                lane.highlight_as_connected(self.id, world);
            }
        }
    }

    pub fn stop_debug_connectivity(&self, world: &mut World) {
        for &Location { node, .. } in self.pathfinding.routes.keys() {
            // TODO: ugly: untyped RawID shenanigans
            if node.as_raw().local_broadcast() == LaneID::local_broadcast(world).as_raw() {
                let lane = LaneID::from_raw(node.as_raw());
                lane.stop_highlight_as_connected(self.id, world);
            }
        }
    }

    pub fn highlight_as_connected(&mut self, for_lane: LaneID, _: &mut World) {
        self.pathfinding.debug_highlight_for.insert(for_lane, ());
    }

    pub fn stop_highlight_as_connected(&mut self, for_lane: LaneID, _: &mut World) {
        self.pathfinding.debug_highlight_for.remove(for_lane);
    }
}

use time::TimeID;

pub fn setup(system: &mut ActorSystem) {
    trip::setup(system);
    auto_setup(system);
}

pub fn spawn(world: &mut World, time: TimeID) {
    trip::spawn(world, time);
}

mod kay_auto;
pub use self::kay_auto::*;
