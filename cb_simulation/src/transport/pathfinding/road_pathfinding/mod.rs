use compact::{CDict, CVec, CHashMap};
use kay::{World, TypedID, Actor};
use transport::lane::{Lane, LaneID};
use transport::lane::connectivity::Interaction;

// TODO: MAKE TRANSFER LANE NOT PARTICIPATE AT ALL IN PATHFINDING -> MUCH SIMPLER

use super::{ROUTING_TIMEOUT_AFTER_CHANGE, PathfindingCore, Node, NodeID, Location, MIN_LANDMARK_INCOMING, DistanceRequesterID, AttacheeID,
RoutingInfo,RoughLocation, IDEAL_LANDMARK_RADIUS, RoughLocationResolve, PreciseLocation, RoughLocationID};
use super::trip::{TripResult, TripFate};

const LOG_T: &str = "Road Pathfinding";

use log::{debug};

pub fn on_connect(lane: &mut Lane) {
    lane.pathfinding.routing_timeout = ROUTING_TIMEOUT_AFTER_CHANGE;
}

pub fn on_disconnect(lane: &mut Lane) {
    lane.pathfinding.routes = CHashMap::new();
    lane.pathfinding.routes_changed = true;
    lane.pathfinding.query_routes_next_tick = true;
}

impl Node for Lane {
    fn core(&self) -> &PathfindingCore {
        &self.pathfinding
    }

    fn core_mut(&mut self) -> &mut PathfindingCore {
        &mut self.pathfinding
    }

    fn update_routes(&mut self, world: &mut World) {
        if let Some(location) = self.pathfinding.location {
            for (successor, _) in successors(self) {
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
            self.pathfinding = PathfindingCore {
                location: Some(Location::landmark(self.id_as())),
                hops_from_landmark: 0,
                learned_landmark_from: Some(self.id_as()),
                routes: CHashMap::new(),
                routes_changed: true,
                query_routes_next_tick: false,
                tell_to_forget_next_tick: CVec::new(),
                routing_timeout: ROUTING_TIMEOUT_AFTER_CHANGE,
                attachees: self.pathfinding.attachees.clone(),
            }
        }

        if self.pathfinding.routing_timeout > 0 {
            self.pathfinding.routing_timeout -= 1;
        } else {
            if self.pathfinding.query_routes_next_tick {
                for (successor, custom_connection_cost) in successors(self) {
                    successor.query_routes(self.id_as(), custom_connection_cost, world);
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

    fn query_routes(&mut self, requester: NodeID, custom_connection_cost: Option<f32>, world: &mut World) {
        let self_cost = custom_connection_cost.unwrap_or(self.construction.length);

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
                    let partner_as_node: NodeID = interaction.indirect_lane_partner().into();
                    partner_as_node == from
                })
        {
            for (&destination, &(new_distance, new_distance_hops)) in new_routes.pairs() {
                if destination.is_landmark()
                    || new_distance_hops <= IDEAL_LANDMARK_RADIUS
                    || self
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
                        hops_from_landmark < self.pathfinding.hops_from_landmark
                            || self
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

            self.pathfinding = PathfindingCore {
                location: Some(join_as),
                learned_landmark_from: Some(from),
                hops_from_landmark,
                routes: CHashMap::new(),
                routes_changed: true,
                query_routes_next_tick: true,
                tell_to_forget_next_tick,
                routing_timeout: ROUTING_TIMEOUT_AFTER_CHANGE,
                attachees: self.pathfinding.attachees.clone(),
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
fn successors<'a>(lane: &'a Lane) -> impl Iterator<Item = (NodeID, Option<f32>)> + 'a {
    lane.connectivity
        .interactions
        .iter()
        .filter_map(|interaction| match *interaction {
            Interaction::Switch {
                to,
                is_left,
                ..
            } => Some((to.into(), Some(if is_left {LANE_CHANGE_COST_LEFT} else {LANE_CHANGE_COST_RIGHT}))),
            Interaction::Next {
                next,
                ..
            } => Some((next.into(), None)),
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
            Interaction::Switch {
                to, 
                ..
            } => Some((i as u8, to.into(), true)),
            // TODO: ugly: untyped RawID shenanigans
            Interaction::Previous {
                previous,
                ..
            } => Some((i as u8, previous.into(), false)),
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

const LANE_CHANGE_COST_LEFT: f32 = 5.0;
const LANE_CHANGE_COST_RIGHT: f32 = 3.0;



mod kay_auto;
pub use self::kay_auto::*;