use kay::{World, Actor};
use transport::lane::{CarLane, CarLaneID, Sidewalk, SidewalkID};
use transport::lane::connectivity::{CarLaneInteraction, SidewalkInteraction};

use super::{
    PathfindingCore, Link, LinkID, Location, LinkConnection, CommunicatedRoutingEntry,
    RoughLocation, RoughLocationResolve, PreciseLocation, RoughLocationID,
};
use super::trip::{TripResult, TripFate};

impl Link for CarLane {
    fn core(&self) -> &PathfindingCore {
        &self.pathfinding
    }

    fn core_mut(&mut self) -> &mut PathfindingCore {
        &mut self.pathfinding
    }

    fn self_as_route(&self) -> Option<(Location, CommunicatedRoutingEntry)> {
        if self.connectivity.on_intersection {
            None
        } else {
            self.core().location.map(|destination| {
                (
                    destination,
                    CommunicatedRoutingEntry {
                        distance: self.construction.length,
                        distance_hops: 0,
                    },
                )
            })
        }
    }

    fn can_be_landmark(&self) -> bool {
        !self.connectivity.on_intersection
    }

    fn map_connected_link_to_idx(&self, link: LinkID) -> Option<usize> {
        self.connectivity
            .interactions
            .iter()
            .position(|interaction| {
                let partner_as_link: LinkID = interaction.indirect_lane_partner().into();
                partner_as_link == link
            })
    }

    fn successors(&self) -> Vec<LinkConnection> {
        self.connectivity
            .interactions
            .iter()
            .filter_map(|interaction| match *interaction {
                CarLaneInteraction::Switch { to, is_left, .. } => Some(LinkConnection {
                    link: to.into(),
                    connection_cost: if is_left {
                        LANE_CHANGE_COST_LEFT
                    } else {
                        LANE_CHANGE_COST_RIGHT
                    },
                }),
                CarLaneInteraction::Next { next, .. } => Some(LinkConnection {
                    link: next.into(),
                    connection_cost: self.construction.length,
                }),
                _ => None,
            })
            .collect()
    }

    fn predecessors(&self) -> Vec<LinkConnection> {
        self.connectivity
            .interactions
            .iter()
            .filter_map(|interaction| match *interaction {
                CarLaneInteraction::Switch { to, is_left, .. } => Some(LinkConnection {
                    link: to.into(),
                    connection_cost: if is_left {
                        LANE_CHANGE_COST_RIGHT
                    } else {
                        LANE_CHANGE_COST_LEFT
                    },
                }),
                CarLaneInteraction::Previous { previous, .. } => Some(LinkConnection {
                    link: previous.into(),
                    connection_cost: self.construction.length,
                }),
                _ => None,
            })
            .collect()
    }

    fn after_route_forgotten(&mut self, forgotten_route: Location, world: &mut World) {
        let self_as_rough_location = self.id_as();

        self.microtraffic.cars.retain(|car| {
            let car_was_going_there = if forgotten_route.is_landmark() {
                car.destination.landmark == forgotten_route.landmark
            } else {
                car.destination.location == forgotten_route
            };

            if car_was_going_there {
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
        });
    }
}

pub fn on_unbuild(lane: &CarLane, world: &mut World) {
    for attachee in &lane.pathfinding.attachees {
        attachee.location_changed(lane.pathfinding.location, None, world);
    }
}

impl RoughLocation for CarLane {
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

impl Link for Sidewalk {
    fn core(&self) -> &PathfindingCore {
        &self.pathfinding
    }

    fn core_mut(&mut self) -> &mut PathfindingCore {
        &mut self.pathfinding
    }

    fn self_as_route(&self) -> Option<(Location, CommunicatedRoutingEntry)> {
        if self.connectivity.on_intersection {
            None
        } else {
            self.core().location.map(|destination| {
                (
                    destination,
                    CommunicatedRoutingEntry {
                        distance: self.construction.length,
                        distance_hops: 0,
                    },
                )
            })
        }
    }

    fn can_be_landmark(&self) -> bool {
        !self.connectivity.on_intersection
    }

    fn map_connected_link_to_idx(&self, link: LinkID) -> Option<usize> {
        self.connectivity
            .interactions
            .iter()
            .position(|interaction| match interaction {
                SidewalkInteraction::Next { next, .. } => Into::<LinkID>::into(*next) == link,
                SidewalkInteraction::Previous { previous, .. } => {
                    Into::<LinkID>::into(*previous) == link
                }
                _ => false,
            })
    }

    fn successors(&self) -> Vec<LinkConnection> {
        self.connectivity
            .interactions
            .iter()
            .filter_map(|interaction| match *interaction {
                SidewalkInteraction::Next { next, .. } => Some(LinkConnection {
                    link: next.into(),
                    connection_cost: self.construction.length,
                }),
                _ => None,
            })
            .collect()
    }

    fn predecessors(&self) -> Vec<LinkConnection> {
        self.connectivity
            .interactions
            .iter()
            .filter_map(|interaction| match *interaction {
                SidewalkInteraction::Previous { previous, .. } => Some(LinkConnection {
                    link: previous.into(),
                    connection_cost: self.construction.length,
                }),
                _ => None,
            })
            .collect()
    }

    fn after_route_forgotten(&mut self, forgotten_route: Location, world: &mut World) {
        let self_as_rough_location = self.id_as();

        self.microtraffic.pedestrians.retain(|pedestrian| {
            let car_was_going_there = if forgotten_route.is_landmark() {
                pedestrian.destination.landmark == forgotten_route.landmark
            } else {
                pedestrian.destination.location == forgotten_route
            };

            if car_was_going_there {
                pedestrian.trip.finish(
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
        });
    }
}

impl RoughLocation for Sidewalk {
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

mod kay_auto;
pub use self::kay_auto::*;
