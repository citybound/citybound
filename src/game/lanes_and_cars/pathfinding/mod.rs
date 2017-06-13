use compact::{CDict, CVec};
use kay::{ID, ActorSystem, Fate, World};
use kay::swarm::{Swarm, SubActor};
use stagemaster::geometry::AnyShape;
use descartes::Band;
use super::lane::{Lane, TransferLane};
use super::connectivity::{Interaction, InteractionKind, OverlapKind};
use core::simulation::Timestamp;

pub mod trip;

#[derive(Compact, Clone, Default)]
pub struct PathfindingInfo {
    pub as_destination: Option<Destination>,
    pub hops_from_landmark: u8,
    pub learned_landmark_from: Option<ID>,
    pub routes: CDict<Destination, RoutingInfo>,
    pub routes_changed: bool,
    pub tell_to_forget_next_tick: CVec<Destination>,
    pub query_routes_next_tick: bool,
    pub routing_timeout: u16,
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct Destination {
    pub landmark: ID,
    pub node: ID,
}

impl Destination {
    fn landmark(landmark: ID) -> Self {
        Destination { landmark: landmark, node: landmark }
    }
    pub fn is_landmark(&self) -> bool {
        self.landmark == self.node
    }
    pub fn landmark_destination(&self) -> Self {
        Self::landmark(self.landmark)
    }
}

#[derive(Copy, Clone, Debug)]
pub struct RoutingInfo {
    pub outgoing_idx: u8,
    pub distance: f32,
    distance_hops: u8,
    learned_from: ID,
    fresh: bool,
}

use stagemaster::{UserInterface, AddInteractable};
const DEBUG_CARS_ON_LANES: bool = false;

pub fn on_build(lane: &mut Lane, world: &mut World) {
    lane.pathfinding.as_destination = None;
    if DEBUG_CARS_ON_LANES {
        world.send_to_id_of::<UserInterface, _>(
                   AddInteractable(lane.id(),
                                   AnyShape::Band(Band::new(lane.construction.path.clone(), 3.0)),
                                   5));
    }
}

pub fn on_connect(lane: &mut Lane) {
    lane.pathfinding.routing_timeout = ROUTING_TIMEOUT_AFTER_CHANGE;
}

pub fn on_disconnect(lane: &mut Lane, disconnected_id: ID) {
    let new_routes = lane.pathfinding
        .routes
        .pairs()
        .filter_map(|(destination, route)| if route.learned_from == disconnected_id {
                        None
                    } else {
                        Some((*destination, *route))
                    })
        .collect();
    lane.pathfinding.routes = new_routes;
    lane.pathfinding.routes_changed = true;
    lane.pathfinding.query_routes_next_tick = true;
}

const MIN_LANDMARK_INCOMING: usize = 3;
const ROUTING_TIMEOUT_AFTER_CHANGE: u16 = 15;

pub fn tick(lane: &mut Lane, world: &mut World) {
    if let Some(as_destination) = lane.pathfinding.as_destination {
        for successor in successors(lane) {
            world.send(successor,
                       JoinLandmark {
                           from: lane.id(),
                           join_as: Destination {
                               landmark: as_destination.landmark,
                               node: successor,
                           },
                           hops_from_landmark: lane.pathfinding.hops_from_landmark + 1,
                       })
        }
    } else if !lane.connectivity.on_intersection &&
              predecessors(lane).count() >= MIN_LANDMARK_INCOMING {
        lane.pathfinding = PathfindingInfo {
            as_destination: Some(Destination::landmark(lane.id())),
            hops_from_landmark: 0,
            learned_landmark_from: Some(lane.id()),
            routes: CDict::new(),
            routes_changed: true,
            query_routes_next_tick: false,
            tell_to_forget_next_tick: CVec::new(),
            routing_timeout: ROUTING_TIMEOUT_AFTER_CHANGE,
        }
    }

    if lane.pathfinding.routing_timeout > 0 {
        lane.pathfinding.routing_timeout -= 1;
    } else {
        if lane.pathfinding.query_routes_next_tick {
            for successor in successors(lane) {
                world.send(successor,
                           QueryRoutes { requester: lane.id(), is_transfer: false });
            }
            lane.pathfinding.query_routes_next_tick = false;
        }

        if !lane.pathfinding.tell_to_forget_next_tick.is_empty() {
            for (_, predecessor, _) in predecessors(lane) {
                world.send(predecessor,
                           ForgetRoutes {
                               forget: lane.pathfinding.tell_to_forget_next_tick.clone(),
                               from: lane.id(),
                           })
            }
            lane.pathfinding.tell_to_forget_next_tick.clear();
        }

        if lane.pathfinding.routes_changed {
            for (_, predecessor, is_transfer) in predecessors(lane) {
                let self_cost = if is_transfer {
                    0.0
                } else {
                    lane.construction.length
                };
                world.send(
                    predecessor,
                    ShareRoutes {
                        new_routes: lane.pathfinding
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
                                if lane.connectivity.on_intersection {
                                    None
                                } else {
                                    lane.pathfinding
                                        .as_destination
                                        .map(|destination| (destination, (self_cost, 0)))
                                },
                            )
                            .collect(),
                        from: lane.id(),
                    },
                );
            }
            for routing_info in lane.pathfinding.routes.values_mut() {
                routing_info.fresh = false;
            }
            lane.pathfinding.routes_changed = false;
        }
    }
}

#[allow(needless_lifetimes)]
fn successors<'a>(lane: &'a Lane) -> impl Iterator<Item = ID> + 'a {
    lane.connectivity
        .interactions
        .iter()
        .filter_map(|interaction| match *interaction {
                        Interaction {
                            partner_lane,
                            kind: InteractionKind::Overlap { kind: OverlapKind::Transfer, .. },
                            ..
                        } |
                        Interaction {
                            partner_lane,
                            kind: InteractionKind::Next { .. },
                            ..
                        } => Some(partner_lane),
                        _ => None,
                    })
}

#[allow(needless_lifetimes)]
fn predecessors<'a>(lane: &'a Lane) -> impl Iterator<Item = (u8, ID, bool)> + 'a {
    lane.connectivity
        .interactions
        .iter()
        .enumerate()
        .filter_map(|(i, interaction)| match *interaction {
                        Interaction {
                            partner_lane,
                            kind: InteractionKind::Overlap { kind: OverlapKind::Transfer, .. },
                            ..
                        } => Some((i as u8, partner_lane, true)),
                        Interaction {
                            partner_lane,
                            kind: InteractionKind::Previous { .. },
                            ..
                        } => Some((i as u8, partner_lane, false)),
                        _ => None,
                    })
}

#[derive(Copy, Clone)]
pub struct JoinLandmark {
    from: ID,
    join_as: Destination,
    hops_from_landmark: u8,
}

const IDEAL_LANDMARK_RADIUS: u8 = 3;

pub fn setup(system: &mut ActorSystem) {
    system.extend(Swarm::<Lane>::subactors(|mut each_lane| {
        each_lane.on(|&JoinLandmark { join_as, hops_from_landmark, from },
         lane,
         _| {
            let join = lane.pathfinding
                .as_destination
                .map(|self_destination| {
                    join_as != self_destination &&
                    (if self_destination.is_landmark() {
                         hops_from_landmark < IDEAL_LANDMARK_RADIUS &&
                         join_as.landmark.sub_actor_id < lane.id().sub_actor_id
                     } else {
                         hops_from_landmark < lane.pathfinding.hops_from_landmark ||
                         lane.pathfinding
                             .learned_landmark_from
                             .map(|learned_from| learned_from == from)
                             .unwrap_or(false)
                     })
                })
                .unwrap_or(true);
            if join {
                lane.pathfinding = PathfindingInfo {
                    as_destination: Some(join_as),
                    learned_landmark_from: Some(from),
                    hops_from_landmark: hops_from_landmark,
                    routes: CDict::new(),
                    routes_changed: true,
                    query_routes_next_tick: true,
                    tell_to_forget_next_tick: lane.pathfinding
                        .routes
                        .keys()
                        .cloned()
                        .chain(lane.pathfinding.as_destination.into_iter())
                        .collect(),
                    routing_timeout: ROUTING_TIMEOUT_AFTER_CHANGE,
                };
            }
            Fate::Live
        });

        each_lane.on(|&QueryRoutes { requester, is_transfer }, lane, world| {
            let self_cost = if is_transfer {
                0.0
            } else {
                lane.construction.length
            };
            world.send(
                requester,
                ShareRoutes {
                    new_routes: lane.pathfinding
                        .routes
                        .pairs()
                        .map(|(&destination,
                          &RoutingInfo { distance, distance_hops, .. })| {
                            (destination, (distance + self_cost, distance_hops + 1))
                        })
                        .chain(
                            if lane.connectivity.on_intersection {
                                None
                            } else {
                                lane.pathfinding
                                    .as_destination
                                    .map(|destination| (destination, (self_cost, 0)))
                            },
                        )
                        .collect(),
                    from: lane.id(),
                },
            );
            Fate::Live
        });

        each_lane.on(|&ShareRoutes { ref new_routes, from }, lane, _| {
            if let Some(from_interaction_idx) =
                lane.connectivity
                    .interactions
                    .iter()
                    .position(|interaction| interaction.partner_lane == from) {
                for (&destination, &(new_distance, new_distance_hops)) in new_routes.pairs() {
                    if destination.is_landmark() || new_distance_hops <= IDEAL_LANDMARK_RADIUS ||
                       lane.pathfinding
                           .as_destination
                           .map(|self_dest| self_dest.landmark == destination.landmark)
                           .unwrap_or(false) {
                        let insert = lane.pathfinding
                            .routes
                            .get_mru(destination)
                            .map(|&RoutingInfo { distance, .. }| new_distance < distance)
                            .unwrap_or(true);
                        if insert {
                            lane.pathfinding.routes.insert(destination,
                                                           RoutingInfo {
                                                               distance: new_distance,
                                                               distance_hops: new_distance_hops,
                                                               outgoing_idx: from_interaction_idx as
                                                                             u8,
                                                               learned_from: from,
                                                               fresh: true,
                                                           });
                            lane.pathfinding.routes_changed = true;
                        }
                    }
                }
            } else {
                println!("{:?} not yet connected to {:?}", lane.id(), from);
            }
            Fate::Live
        });

        each_lane.on(|&ForgetRoutes { ref forget, from }, lane, _| {
            let mut forgotten = CVec::<Destination>::new();
            for destination_to_forget in forget.iter() {
                let forget = if let Some(routing_info) =
                    lane.pathfinding.routes.get(*destination_to_forget) {
                    routing_info.learned_from == from
                } else {
                    false
                };
                if forget {
                    lane.pathfinding.routes.remove(*destination_to_forget);
                    if destination_to_forget.is_landmark() {
                        lane.microtraffic.cars.retain(|car| {
                            car.destination.landmark != destination_to_forget.landmark
                        })
                    } else {
                        lane.microtraffic
                            .cars
                            .retain(|car| &car.destination != destination_to_forget)
                    }
                    forgotten.push(*destination_to_forget);
                }
            }
            lane.pathfinding.tell_to_forget_next_tick = forgotten;
            Fate::Live
        });

        each_lane.on(|&QueryAsDestination {
             tick,
             requester,
             rough_destination,
         },
         lane,
         world| {
            world.send(requester,
                       TellAsDestination {
                           tick,
                           rough_destination,
                           as_destination: lane.pathfinding.as_destination,
                       });
            Fate::Live
        });

        each_lane.on(|&GetDistanceTo { destination, requester }, lane, world| {
            let maybe_distance = lane.pathfinding
                .routes
                .get(destination)
                .map(|routing_info| routing_info.distance);
            world.send(requester, DistanceInfo(maybe_distance));
            Fate::Live
        });
    }));

    system.extend(Swarm::<TransferLane>::subactors(|mut each_t_lane| {
        each_t_lane.on(|&JoinLandmark { join_as, hops_from_landmark, from },
         lane,
         world| {
            world.send(lane.other_side(from),
                       JoinLandmark {
                           join_as: Destination {
                               landmark: join_as.landmark,
                               node: lane.other_side(from),
                           },
                           hops_from_landmark: hops_from_landmark,
                           from: lane.id(),
                       });
            Fate::Live
        });

        each_t_lane.on(|&QueryRoutes { requester, .. }, lane, world| {
            world.send(lane.other_side(requester),
                       QueryRoutes { requester: lane.id(), is_transfer: true });
            Fate::Live
        });

        each_t_lane.on(|&ShareRoutes { ref new_routes, from }, lane, world| {
            world.send(
                lane.other_side(from),
                ShareRoutes {
                    new_routes: new_routes
                        .pairs()
                        .map(|(&destination, &(distance, hops))| {
                            (destination,
                             (distance +
                              if from == lane.connectivity.left.expect("should have left").0 {
                                  LANE_CHANGE_COST_RIGHT
                              } else {
                                  LANE_CHANGE_COST_LEFT
                              },
                              hops))
                        })
                        .collect(),
                    from: lane.id(),
                },
            );
            Fate::Live
        });

        each_t_lane.on(|&ForgetRoutes { ref forget, from }, lane, world| {
            world.send(lane.other_side(from),
                       ForgetRoutes { forget: forget.clone(), from: lane.id() });
            Fate::Live
        })
    }));

    trip::setup(system);
}

#[derive(Copy, Clone)]
pub struct QueryRoutes {
    requester: ID,
    is_transfer: bool,
}

#[derive(Compact, Clone)]
pub struct ShareRoutes {
    new_routes: CDict<Destination, (f32, u8)>,
    from: ID,
}

const LANE_CHANGE_COST_LEFT: f32 = 5.0;
const LANE_CHANGE_COST_RIGHT: f32 = 3.0;

#[derive(Compact, Clone)]
pub struct ForgetRoutes {
    forget: CVec<Destination>,
    from: ID,
}

#[derive(Copy, Clone)]
pub struct QueryAsDestination {
    pub requester: ID,
    pub rough_destination: ID,
    pub tick: Option<Timestamp>,
}
#[derive(Copy, Clone)]
pub struct TellAsDestination {
    pub rough_destination: ID,
    pub as_destination: Option<Destination>,
    pub tick: Option<Timestamp>,
}

#[derive(Copy, Clone)]
pub struct GetDistanceTo {
    pub destination: Destination,
    pub requester: ID,
}

#[derive(Copy, Clone)]
pub struct DistanceInfo(pub Option<f32>);
