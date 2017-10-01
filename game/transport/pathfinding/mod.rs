use compact::{CDict, CVec, CHashMap};
use kay::{ActorSystem, World};
use super::lane::{Lane, LaneID, TransferLane, TransferLaneID};
use super::lane::connectivity::{Interaction, InteractionKind, OverlapKind};
use core::simulation::Timestamp;

// TODO: MAKE TRANSFER LANE NOT PARTICIPATE AT ALL IN PATHFINDING -> MUCH SIMPLER

pub mod trip;

pub trait Node {
    fn update_routes(&mut self, world: &mut World);
    fn query_routes(&mut self, requester: NodeID, is_transfer: bool, world: &mut World);
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
}

#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub struct Location {
    pub landmark: NodeID,
    pub node: NodeID,
}

impl Location {
    fn landmark(landmark: NodeID) -> Self {
        Location { landmark: landmark, node: landmark }
    }
    pub fn is_landmark(&self) -> bool {
        self.landmark == self.node
    }
    pub fn landmark_destination(&self) -> Self {
        Self::landmark(self.landmark)
    }
}

#[derive(Copy, Clone)]
pub struct RoutingInfo {
    pub outgoing_idx: u8,
    pub distance: f32,
    distance_hops: u8,
    learned_from: NodeID,
    fresh: bool,
}

// use stagemaster::{UserInterfaceID, MSG};
// const DEBUG_CARS_ON_LANES: bool = false;

// pub fn on_build(lane: &mut Lane, world: &mut World) {
//     lane.pathfinding.as_destination = None;
//     if DEBUG_CARS_ON_LANES {
//         world.send_to_id_of::<UserInterface, _>(AddInteractable(
//             lane.id(),
//             AnyShape::Band(
//                 Band::new(lane.construction.path.clone(), 3.0),
//             ),
//             5,
//         ));
//     }
// }

pub fn on_connect(lane: &mut Lane) {
    lane.pathfinding.routing_timeout = ROUTING_TIMEOUT_AFTER_CHANGE;
}

use super::microtraffic::LaneLikeID;

pub fn on_disconnect(lane: &mut Lane, disconnected_id: LaneLikeID) {
    let new_routes = lane.pathfinding
        .routes
        .pairs()
        // TODO: ugly: untyped ID shenanigans
        .filter_map(|(destination, route)| if route.learned_from._raw_id ==
            disconnected_id._raw_id
        {
            None
        } else {
            Some((*destination, *route))
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
                    self.id.into(),
                    Location {
                        landmark: location.landmark,
                        node: successor,
                    },
                    self.pathfinding.hops_from_landmark + 1,
                    world,
                );
            }
        } else if !self.connectivity.on_intersection &&
                   predecessors(self).count() >= MIN_LANDMARK_INCOMING
        {
            self.pathfinding = PathfindingInfo {
                location: Some(Location::landmark(self.id.into())),
                hops_from_landmark: 0,
                learned_landmark_from: Some(self.id.into()),
                routes: CHashMap::new(),
                routes_changed: true,
                query_routes_next_tick: false,
                tell_to_forget_next_tick: CVec::new(),
                routing_timeout: ROUTING_TIMEOUT_AFTER_CHANGE,
            }
        }

        if self.pathfinding.routing_timeout > 0 {
            self.pathfinding.routing_timeout -= 1;
        } else {
            if self.pathfinding.query_routes_next_tick {
                for successor in successors(self) {
                    successor.query_routes(self.id.into(), false, world);
                }
                self.pathfinding.query_routes_next_tick = false;
            }

            if !self.pathfinding.tell_to_forget_next_tick.is_empty() {
                for (_, predecessor, _) in predecessors(self) {
                    predecessor.forget_routes(
                        self.pathfinding.tell_to_forget_next_tick.clone(),
                        self.id.into(),
                        world,
                    );
                }
                self.pathfinding.tell_to_forget_next_tick.clear();
            }

            if self.pathfinding.routes_changed {
                for (_, predecessor, is_transfer) in predecessors(self) {
                    let self_cost = if is_transfer {
                        0.0
                    } else {
                        self.construction.length
                    };
                    predecessor.on_routes(self.pathfinding
                            .routes
                            .pairs()
                            .filter_map(|(&destination,
                              &RoutingInfo { distance, distance_hops, .. })| {
                                if true
                                // fresh
                                {
                                    Some((destination, (distance + self_cost, distance_hops + 1)))
                                } else {
                                    None
                                }
                            })
                            .chain(
                                if self.connectivity.on_intersection {
                                    None
                                } else {
                                    self.pathfinding
                                        .location
                                        .map(|destination| (destination, (self_cost, 0)))
                                },
                            )
                            .collect(), self.id.into(), world);
                }
                for routing_info in self.pathfinding.routes.values_mut() {
                    routing_info.fresh = false;
                }
                self.pathfinding.routes_changed = false;
            }
        }
    }

    fn query_routes(&mut self, requester: NodeID, is_transfer: bool, world: &mut World) {
        let self_cost = if is_transfer {
            0.0
        } else {
            self.construction.length
        };
        requester.on_routes(
            self.pathfinding
                .routes
                .pairs()
                .map(|(&destination,
                  &RoutingInfo { distance, distance_hops, .. })| {
                    (destination, (distance + self_cost, distance_hops + 1))
                })
                .chain(if self.connectivity.on_intersection {
                    None
                } else {
                    self.pathfinding.location.map(|destination| {
                        (destination, (self_cost, 0))
                    })
                })
                .collect(),
            self.id.into(),
            world,
        );
    }

    fn on_routes(&mut self, new_routes: &CDict<Location, (f32, u8)>, from: NodeID, _: &mut World) {
        if let Some(from_interaction_idx) =
            self.connectivity.interactions.iter().position(
                |interaction| {
                    // TODO: ugly: untyped ID shenanigans
                    interaction.partner_lane._raw_id == from._raw_id
                },
            )
        {
            for (&destination, &(new_distance, new_distance_hops)) in new_routes.pairs() {
                if destination.is_landmark() || new_distance_hops <= IDEAL_LANDMARK_RADIUS ||
                    self.pathfinding
                        .location
                        .map(|self_dest| self_dest.landmark == destination.landmark)
                        .unwrap_or(false)
                {
                    let insert = self.pathfinding
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
            println!(
                "{:?} not yet connected to {:?}",
                self.id._raw_id,
                from._raw_id
            );
        }
    }

    fn forget_routes(&mut self, forget: &CVec<Location>, from: NodeID, _: &mut World) {
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
                if destination_to_forget.is_landmark() {
                    self.microtraffic.cars.retain(|car| {
                        car.destination.landmark != destination_to_forget.landmark
                    })
                } else {
                    self.microtraffic.cars.retain(|car| {
                        &car.destination != destination_to_forget
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
        _: &mut World,
    ) {
        let join = self.pathfinding
            .location
            .map(|self_destination| {
                join_as != self_destination &&
                    (if self_destination.is_landmark() {
                         hops_from_landmark < IDEAL_LANDMARK_RADIUS &&
                             join_as.landmark._raw_id.instance_id < self.id._raw_id.instance_id
                     } else {
                         hops_from_landmark < self.pathfinding.hops_from_landmark ||
                             self.pathfinding
                                 .learned_landmark_from
                                 .map(|learned_from| learned_from == from)
                                 .unwrap_or(false)
                     })
            })
            .unwrap_or(true);
        if join {
            let tell_to_forget_next_tick = self.pathfinding
                .routes
                .keys()
                .cloned()
                .chain(self.pathfinding.location.into_iter())
                .collect();
            self.pathfinding = PathfindingInfo {
                location: Some(join_as),
                learned_landmark_from: Some(from),
                hops_from_landmark: hops_from_landmark,
                routes: CHashMap::new(),
                routes_changed: true,
                query_routes_next_tick: true,
                tell_to_forget_next_tick: tell_to_forget_next_tick,
                routing_timeout: ROUTING_TIMEOUT_AFTER_CHANGE,
            };
        }
    }

    fn get_distance_to(
        &mut self,
        destination: Location,
        requester: DistanceRequesterID,
        world: &mut World,
    ) {
        let maybe_distance = self.pathfinding
            .routes
            .get(destination)
            .or_else(|| {
                self.pathfinding.routes.get(
                    destination.landmark_destination(),
                )
            })
            .map(|routing_info| routing_info.distance);
        requester.on_distance(maybe_distance, world);
    }
}


#[allow(needless_lifetimes)]
fn successors<'a>(lane: &'a Lane) -> impl Iterator<Item = NodeID> + 'a {
    lane.connectivity.interactions.iter().filter_map(
        |interaction| {
            match *interaction {
                Interaction {
                    partner_lane,
                    kind: InteractionKind::Overlap { kind: OverlapKind::Transfer, .. },
                    ..
                } |
                Interaction {
                    partner_lane,
                    kind: InteractionKind::Next { .. },
                    ..
                } => Some(NodeID { _raw_id: partner_lane._raw_id }), // TODO: ugly: untyped ID shenanigans
                _ => None,
            }
        },
    )
}

#[allow(needless_lifetimes)]
fn predecessors<'a>(lane: &'a Lane) -> impl Iterator<Item = (u8, NodeID, bool)> + 'a {
    lane.connectivity
        .interactions
        .iter()
        .enumerate()
        .filter_map(|(i, interaction)| match *interaction {
            Interaction {
                partner_lane,
                kind: InteractionKind::Overlap { kind: OverlapKind::Transfer, .. },
                ..
            } => Some((i as u8, NodeID{_raw_id: partner_lane._raw_id}, true)), // TODO: ugly: untyped ID shenanigans
            Interaction {
                partner_lane,
                kind: InteractionKind::Previous { .. },
                ..
            } => Some((i as u8, NodeID{_raw_id: partner_lane._raw_id}, false)), // TODO: ugly: untyped ID shenanigans
            _ => None,
        })
}

impl RoughLocation for Lane {
    fn resolve_as_location(
        &mut self,
        requester: LocationRequesterID,
        rough_location: RoughLocationID,
        tick: Timestamp,
        world: &mut World,
    ) {
        requester.location_resolved(rough_location, self.pathfinding.location, tick, world);
    }
}

impl Node for TransferLane {
    fn update_routes(&mut self, _: &mut World) {}

    fn query_routes(&mut self, requester: NodeID, _is_transfer: bool, world: &mut World) {
        // TODO: ugly: untyped ID shenanigans
        let requester_lane = LaneID { _raw_id: requester._raw_id };
        let other_lane: NodeID = self.other_side(requester_lane).into();
        other_lane.query_routes(self.id.into(), true, world);
    }

    fn on_routes(
        &mut self,
        new_routes: &CDict<Location, (f32, u8)>,
        from: NodeID,
        world: &mut World,
    ) {
        let from_lane = LaneID { _raw_id: from._raw_id };
        let other_lane: NodeID = self.other_side(from_lane).into();
        other_lane.on_routes(
            new_routes
                .pairs()
                .map(|(&destination, &(distance, hops))| {
                    // TODO: ugly: untyped ID shenanigans
                    let change_cost = if from._raw_id ==
                        self.connectivity.left.expect("should have left").0._raw_id
                    {
                        LANE_CHANGE_COST_RIGHT
                    } else {
                        LANE_CHANGE_COST_LEFT
                    };
                    (destination, (distance + change_cost, hops))
                })
                .collect(),
            self.id.into(),
            world,
        );
    }

    fn forget_routes(&mut self, forget: &CVec<Location>, from: NodeID, world: &mut World) {
        let from_lane = LaneID { _raw_id: from._raw_id };
        let other_lane: NodeID = self.other_side(from_lane).into();
        other_lane.forget_routes(forget.clone(), self.id.into(), world);
    }

    fn join_landmark(
        &mut self,
        from: NodeID,
        join_as: Location,
        hops_from_landmark: u8,
        world: &mut World,
    ) {
        let from_lane = LaneID { _raw_id: from._raw_id };
        let other_lane: NodeID = self.other_side(from_lane).into();
        other_lane.join_landmark(
            self.id.into(),
            Location {
                landmark: join_as.landmark,
                node: other_lane,
            },
            hops_from_landmark,
            world,
        );
    }

    fn get_distance_to(
        &mut self,
        _location: Location,
        _requester: DistanceRequesterID,
        _: &mut World,
    ) {
        unimplemented!()
    }
}

pub trait RoughLocation {
    fn resolve_as_location(
        &mut self,
        requester: LocationRequesterID,
        rough_location: RoughLocationID,
        tick: Timestamp,
        world: &mut World,
    );
}

pub trait LocationRequester {
    fn location_resolved(
        &mut self,
        rough_location: RoughLocationID,
        location: Option<Location>,
        tick: Timestamp,
        world: &mut World,
    );
}

pub trait DistanceRequester {
    fn on_distance(&mut self, maybe_distance: Option<f32>, world: &mut World);
}

use core::simulation::SimulationID;

pub fn setup(system: &mut ActorSystem, simulation: SimulationID) {
    trip::setup(system, simulation);
    auto_setup(system);
}

mod kay_auto;
pub use self::kay_auto::*;
