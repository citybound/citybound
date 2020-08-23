use kay::{ActorSystem, World, TypedID, Actor, Fate};
use compact::CVec;
use ordered_float::OrderedFloat;
use std::f32::INFINITY;
use std::ops::{Deref, DerefMut};

use super::lane::{CarLane, CarLaneID, CarSwitchLane, CarSwitchLaneID, Sidewalk, SidewalkID};
use super::lane::connectivity::{CarLaneInteraction, SidewalkInteraction};
use super::pathfinding;

mod intelligent_acceleration;
use self::intelligent_acceleration::intelligent_acceleration;
use descartes::{Band, LinePath, RoughEq, Intersect, N, WithUniqueOrthogonal, Segment};
use itertools::Itertools;

use cb_util::log::debug;
const LOG_T: &str = "Microtraffic";

// TODO: move all iteration, updates, etc into one huge retain loop (see identical TODO below)

#[derive(Compact, Clone)]
pub struct CarMicrotraffic {
    pub obstacles: CVec<(Obstacle, ObstacleContainerID)>,
    pub cars: CVec<LaneCar>,
    timings: CVec<bool>,
    pub green: bool,
    pub yellow_to_green: bool,
    pub yellow_to_red: bool,
}

impl CarMicrotraffic {
    pub fn new(timings: CVec<bool>) -> Self {
        CarMicrotraffic {
            obstacles: CVec::new(),
            cars: CVec::new(),
            timings,
            green: false,
            yellow_to_green: false,
            yellow_to_red: false,
        }
    }
}

// makes "time pass slower" for traffic, so we can still use realistic
// unit values while traffic happening at a slower pace to be visible
const MICROTRAFFIC_UNREALISTIC_SLOWDOWN: f32 = 6.0;

#[derive(Compact, Clone, Default)]
pub struct CarSwitchMicrotraffic {
    pub left_obstacles: CVec<Obstacle>,
    pub right_obstacles: CVec<Obstacle>,
    pub cars: CVec<TransferringLaneCar>,
}

#[derive(Copy, Clone)]
pub struct Obstacle {
    pub position: OrderedFloat<f32>,
    pub velocity: f32,
    pub max_velocity: f32,
}

impl Obstacle {
    fn far_ahead() -> Obstacle {
        Obstacle {
            position: OrderedFloat(INFINITY),
            velocity: INFINITY,
            max_velocity: INFINITY,
        }
    }
    fn offset_by(&self, delta: f32) -> Obstacle {
        Obstacle {
            position: OrderedFloat(*self.position + delta),
            ..*self
        }
    }
}

use super::pathfinding::trip::{TripID, TripResult, TripFate};
use super::pathfinding::Link;

#[derive(Copy, Clone)]
pub struct LaneCar {
    pub trip: TripID,
    pub as_obstacle: Obstacle,
    pub acceleration: f32,
    pub destination: pathfinding::PreciseLocation,
    pub next_hop_interaction: Option<u8>,
}

impl LaneCar {
    fn offset_by(&self, delta: f32) -> LaneCar {
        LaneCar {
            as_obstacle: self.as_obstacle.offset_by(delta),
            ..*self
        }
    }
}

impl Deref for LaneCar {
    type Target = Obstacle;

    fn deref(&self) -> &Obstacle {
        &self.as_obstacle
    }
}

impl DerefMut for LaneCar {
    fn deref_mut(&mut self) -> &mut Obstacle {
        &mut self.as_obstacle
    }
}

#[derive(Copy, Clone)]
pub struct TransferringLaneCar {
    as_lane_car: LaneCar,
    pub switch_position: f32,
    pub switch_velocity: f32,
    pub switch_acceleration: f32,
    cancelling: bool,
}

impl Deref for TransferringLaneCar {
    type Target = LaneCar;

    fn deref(&self) -> &LaneCar {
        &self.as_lane_car
    }
}

impl DerefMut for TransferringLaneCar {
    fn deref_mut(&mut self) -> &mut LaneCar {
        &mut self.as_lane_car
    }
}

use cb_time::units::Instant;

pub trait ObstacleContainer {
    fn start_connecting_overlaps(&mut self, other: &CVec<ObstacleContainerID>, world: &mut World);

    fn connect_overlaps(
        &mut self,
        other_obstacle_container: ObstacleContainerID,
        other_path: &LinePath,
        other_width: N,
        reply_needed: bool,
        world: &mut World,
    );

    fn add_obstacles(
        &mut self,
        obstacles: &CVec<Obstacle>,
        from: ObstacleContainerID,
        world: &mut World,
    );

    fn disconnect(&mut self, other: ObstacleContainerID, world: &mut World);
    fn on_confirm_disconnect(&mut self, world: &mut World) -> Fate;
}

pub trait CarLaneLike {
    fn add_car(
        &mut self,
        car: LaneCar,
        from: Option<CarLaneLikeID>,
        instant: Instant,
        world: &mut World,
    );
}

use self::pathfinding::StoredRoutingEntry;

use cb_time::actors::{Temporal, TemporalID};

const TRAFFIC_LOGIC_THROTTLING: usize = 10;
const PATHFINDING_THROTTLING: usize = 10;

impl CarLaneLike for CarLane {
    fn add_car(
        &mut self,
        car: LaneCar,
        _from: Option<CarLaneLikeID>,
        instant: Instant,
        world: &mut World,
    ) {
        // TODO: does this really happen, or is it ok to only hacke this logic in tick()?
        if let Some(self_as_location) = self.pathfinding.location {
            if car.destination.location == self_as_location
                && *car.position >= car.destination.offset
            {
                car.trip.finish(
                    TripResult {
                        location_now: None,
                        fate: TripFate::Success(instant),
                    },
                    world,
                );

                return;
            }
        }

        let (maybe_next_hop_interaction, almost_there) =
            if Some(car.destination.location) == self.pathfinding.location {
                (None, true)
            } else {
                let maybe_hop = self
                    .pathfinding
                    .routes
                    .get(car.destination.location)
                    .or_else(|| {
                        self.pathfinding
                            .routes
                            .get(car.destination.landmark_destination())
                    })
                    .map(|&StoredRoutingEntry { outgoing_idx, .. }| outgoing_idx as usize);

                (maybe_hop, false)
            };

        if maybe_next_hop_interaction.is_some() || almost_there {
            let routed_car = LaneCar {
                next_hop_interaction: maybe_next_hop_interaction.map(|hop| hop as u8),
                ..car
            };

            // TODO: optimize using BinaryHeap?
            let maybe_next_car_position =
                self.microtraffic.cars.iter().position(|other_car| {
                    other_car.as_obstacle.position > car.as_obstacle.position
                });
            match maybe_next_car_position {
                Some(next_car_position) => {
                    self.microtraffic.cars.insert(next_car_position, routed_car)
                }
                None => self.microtraffic.cars.push(routed_car),
            }
        } else {
            car.trip.finish(
                TripResult {
                    location_now: Some(self.id_as()),
                    fate: TripFate::NoRoute,
                },
                world,
            );
        }
    }
}

const LANE_WIDTH_AS_OBSTACLE_CONTAINER: N = 4.5;

impl ObstacleContainer for CarLane {
    fn start_connecting_overlaps(
        &mut self,
        other_obstacle_containers: &CVec<ObstacleContainerID>,
        world: &mut World,
    ) {
        for &other_obstacle_container in other_obstacle_containers.iter() {
            other_obstacle_container.connect_overlaps(
                self.id_as(),
                self.construction.path.clone(),
                LANE_WIDTH_AS_OBSTACLE_CONTAINER,
                true,
                world,
            );
        }
    }

    fn connect_overlaps(
        &mut self,
        other_id: ObstacleContainerID,
        other_path: &LinePath,
        other_width: N,
        reply_needed: bool,
        world: &mut World,
    ) {
        let &(ref lane_band, ref lane_outline) = {
            let band = Band::new(
                self.construction.path.clone(),
                LANE_WIDTH_AS_OBSTACLE_CONTAINER,
            );
            let outline = band.outline();
            &(band, outline)
        };

        let &(ref other_band, ref other_outline) = {
            let band = Band::new(other_path.clone(), LANE_WIDTH_AS_OBSTACLE_CONTAINER);
            let outline = band.outline();
            &(band, outline)
        };

        let intersections = (lane_outline, other_outline).intersect();
        if intersections.len() >= 2 {
            if let ::itertools::MinMaxResult::MinMax(
                (entry_intersection, entry_distance),
                (exit_intersection, exit_distance),
            ) = intersections
                .iter()
                .map(|intersection| {
                    (
                        intersection,
                        lane_band.outline_distance_to_path_distance(intersection.along_a),
                    )
                })
                .minmax_by_key(|&(_, distance)| OrderedFloat(distance))
            {
                let other_entry_distance =
                    other_band.outline_distance_to_path_distance(entry_intersection.along_b);
                let other_exit_distance =
                    other_band.outline_distance_to_path_distance(exit_intersection.along_b);

                let can_weave = other_path
                    .direction_along(other_entry_distance)
                    .rough_eq_by(self.construction.path.direction_along(entry_distance), 0.1)
                    || other_path
                        .direction_along(other_exit_distance)
                        .rough_eq_by(self.construction.path.direction_along(exit_distance), 0.1);

                self.connectivity
                    .interactions
                    .push(CarLaneInteraction::Conflicting {
                        conflicting: other_id,
                        start: entry_distance,
                        conflicting_start: other_entry_distance.min(other_exit_distance),
                        end: exit_distance,
                        conflicting_end: other_exit_distance.max(other_entry_distance),
                        can_weave,
                    });
            } else {
                panic!("both entry and exit should exist")
            }
        }

        if reply_needed {
            other_id.connect_overlaps(
                self.id_as(),
                self.construction.path.clone(),
                LANE_WIDTH_AS_OBSTACLE_CONTAINER,
                false,
                world,
            );
        }
    }

    fn add_obstacles(
        &mut self,
        obstacles: &CVec<Obstacle>,
        from: ObstacleContainerID,
        _: &mut World,
    ) {
        self.microtraffic
            .obstacles
            .retain(|&(_, received_from)| received_from != from);
        self.microtraffic
            .obstacles
            .extend(obstacles.iter().map(|obstacle| (*obstacle, from)));
    }

    fn disconnect(&mut self, other_id: ObstacleContainerID, world: &mut World) {
        self.connectivity
            .interactions
            .retain(|interaction| interaction.direct_lane_partner() != Some(other_id.into()));

        self.microtraffic.obstacles.drain();

        let self_as_rough_location = self.id_as();

        for car in self.microtraffic.cars.drain() {
            car.trip.finish(
                TripResult {
                    location_now: Some(self_as_rough_location),
                    fate: TripFate::HopDisconnected,
                },
                world,
            );
        }

        ::transport::pathfinding::Link::on_disconnect(self);
        other_id.on_confirm_disconnect(world);
    }

    fn on_confirm_disconnect(&mut self, world: &mut World) -> Fate {
        self.construction.disconnects_remaining -= 1;
        if self.construction.disconnects_remaining == 0 {
            self.finalize(
                self.construction
                    .unbuilding_for
                    .expect("should be unbuilding"),
                world,
            );
            Fate::Die
        } else {
            Fate::Live
        }
    }
}

impl CarLane {
    pub fn on_signal_changed(&mut self, from: CarLaneID, new_green: bool, _: &mut World) {
        for interaction in self.connectivity.interactions.iter_mut() {
            match *interaction {
                CarLaneInteraction::Next {
                    next,
                    ref mut green,
                } if next == from => *green = new_green,
                _ => {}
            }
        }
    }
}

impl Temporal for CarLane {
    fn tick(&mut self, dt: f32, current_instant: Instant, world: &mut World) {
        let dt = dt / MICROTRAFFIC_UNREALISTIC_SLOWDOWN;

        // self.construction.progress += dt * 400.0;

        let do_traffic = current_instant.ticks() % TRAFFIC_LOGIC_THROTTLING
            == self.id.as_raw().instance_id as usize % TRAFFIC_LOGIC_THROTTLING;

        let old_green = self.microtraffic.green;
        self.microtraffic.yellow_to_red = if self.microtraffic.timings.is_empty() {
            true
        } else {
            !self.microtraffic.timings
                [((current_instant.ticks() + 100) / 30) % self.microtraffic.timings.len()]
        };
        self.microtraffic.yellow_to_green = if self.microtraffic.timings.is_empty() {
            true
        } else {
            self.microtraffic.timings
                [((current_instant.ticks() + 100) / 30) % self.microtraffic.timings.len()]
        };
        self.microtraffic.green = if self.microtraffic.timings.is_empty() {
            true
        } else {
            self.microtraffic.timings
                [(current_instant.ticks() / 30) % self.microtraffic.timings.len()]
        };

        // TODO: this is just a hacky way to update new lanes about existing lane's green
        if old_green != self.microtraffic.green || do_traffic {
            for interaction in &self.connectivity.interactions {
                if let CarLaneInteraction::Previous { previous, .. } = *interaction {
                    previous.on_signal_changed(self.id, self.microtraffic.green, world);
                }
            }
        }

        if current_instant.ticks() % PATHFINDING_THROTTLING
            == self.id.as_raw().instance_id as usize % PATHFINDING_THROTTLING
        {
            self.pathfinding_tick(world);
        }

        if do_traffic {
            // TODO: optimize using BinaryHeap?
            self.microtraffic
                .obstacles
                .sort_by_key(|&(ref obstacle, _id)| obstacle.position);

            let mut obstacles = self
                .microtraffic
                .obstacles
                .iter()
                .map(|&(ref obstacle, _id)| obstacle);
            let mut maybe_next_obstacle = obstacles.next();

            for c in 0..self.microtraffic.cars.len() {
                let next_obstacle = self
                    .microtraffic
                    .cars
                    .get(c + 1)
                    .map_or(Obstacle::far_ahead(), |car| car.as_obstacle);
                let car = &mut self.microtraffic.cars[c];
                let next_car_acceleration = intelligent_acceleration(car, &next_obstacle, 2.0);

                maybe_next_obstacle = maybe_next_obstacle.and_then(|obstacle| {
                    let mut following_obstacle = Some(obstacle);
                    while following_obstacle.is_some()
                        && *following_obstacle.unwrap().position < *car.position + 0.1
                    {
                        following_obstacle = obstacles.next();
                    }
                    following_obstacle
                });

                let next_obstacle_acceleration = if let Some(next_obstacle) = maybe_next_obstacle {
                    intelligent_acceleration(car, next_obstacle, 3.0)
                } else {
                    INFINITY
                };

                car.acceleration = next_car_acceleration.min(next_obstacle_acceleration);

                if let Some(next_hop_interaction) = car.next_hop_interaction {
                    if let CarLaneInteraction::Next { green, .. } =
                        self.connectivity.interactions[next_hop_interaction as usize]
                    {
                        if !green {
                            car.acceleration = car.acceleration.min(intelligent_acceleration(
                                car,
                                &Obstacle {
                                    position: OrderedFloat(self.construction.length + 2.0),
                                    velocity: 0.0,
                                    max_velocity: 0.0,
                                },
                                2.0,
                            ))
                        }
                    }
                }
            }
        }

        for car in &mut self.microtraffic.cars {
            *car.position += dt * car.velocity;
            car.velocity = (car.velocity + dt * car.acceleration)
                .min(car.max_velocity)
                .max(0.0);
        }

        for &mut (ref mut obstacle, _id) in &mut self.microtraffic.obstacles {
            *obstacle.position += dt * obstacle.velocity;
        }

        if self.microtraffic.cars.len() > 1 {
            for i in (0..self.microtraffic.cars.len() - 1).rev() {
                self.microtraffic.cars[i].position = OrderedFloat(
                    (*self.microtraffic.cars[i].position)
                        .min(*self.microtraffic.cars[i + 1].position),
                );
            }
        }

        // TODO: move all iteration, updates, etc into one huge retain loop

        if let Some(self_as_location) = self.pathfinding.location {
            self.microtraffic.cars.retain(|car| {
                if car.destination.location == self_as_location
                    && *car.position >= car.destination.offset
                {
                    car.trip.finish(
                        TripResult {
                            location_now: None,
                            fate: TripFate::Success(current_instant),
                        },
                        world,
                    );

                    false
                } else {
                    true
                }
            });
        }

        loop {
            let maybe_switch_car: Option<(usize, CarLaneLikeID, f32)> = self
                .microtraffic
                .cars
                .iter()
                .enumerate()
                .rev()
                .filter_map(|(i, &car)| {
                    let interaction = car.next_hop_interaction.map(|hop_interaction| {
                        self.connectivity.interactions[hop_interaction as usize]
                    });

                    match interaction {
                        Some(CarLaneInteraction::Switch {
                            start, end, via, ..
                        }) => {
                            if *car.position > start && *car.position > end - 300.0 {
                                Some((i, via.into(), start))
                            } else {
                                None
                            }
                        }
                        Some(CarLaneInteraction::Next { next, .. }) => {
                            if *car.position > self.construction.length {
                                Some((i, next.into(), self.construction.length))
                            } else {
                                None
                            }
                        }
                        Some(_) => {
                            unreachable!("Car has a next hop that is neither Next nor Switch")
                        }
                        None => None,
                    }
                })
                .next();

            if let Some((idx_to_remove, next_lane, start)) = maybe_switch_car {
                let car = self.microtraffic.cars.remove(idx_to_remove);
                next_lane.add_car(
                    car.offset_by(-start),
                    Some(self.id_as()),
                    current_instant,
                    world,
                );
            } else {
                break;
            }
        }

        // ASSUMPTION: only one interaction per Lane/Lane pair
        for interaction in self.connectivity.interactions.iter() {
            let cars = self.microtraffic.cars.iter();

            if (current_instant.ticks() + 1) % TRAFFIC_LOGIC_THROTTLING
                == interaction.direct_partner().as_raw().instance_id as usize
                    % TRAFFIC_LOGIC_THROTTLING
            {
                let maybe_obstacles = obstacles_for_interaction(
                    interaction,
                    cars,
                    self.microtraffic.obstacles.iter(),
                );

                if let Some(obstacles) = maybe_obstacles {
                    interaction
                        .direct_partner()
                        .add_obstacles(obstacles, self.id_as(), world);
                }
            }
        }
    }
}

impl CarLaneLike for CarSwitchLane {
    fn add_car(
        &mut self,
        car: LaneCar,
        maybe_from: Option<CarLaneLikeID>,
        _tick: Instant,
        _: &mut World,
    ) {
        let from = maybe_from.expect("car has to come from somewhere on switch lane");

        let from_left = from
            == self
                .connectivity
                .left
                .expect("should have a left lane")
                .0
                .into();
        let side_multiplier = if from_left { -1.0 } else { 1.0 };
        let offset = self.interaction_to_self_offset(*car.position, from_left);
        self.microtraffic.cars.push(TransferringLaneCar {
            as_lane_car: car.offset_by(offset),
            switch_position: 1.0 * side_multiplier,
            switch_velocity: 0.0,
            switch_acceleration: 0.3 * -side_multiplier,
            cancelling: false,
        });
        // TODO: optimize using BinaryHeap?
        self.microtraffic
            .cars
            .sort_by_key(|car| car.as_obstacle.position);
    }
}

use dimensions::{MAX_SWITCHING_LANE_DISTANCE, MIN_SWITCHING_LANE_LENGTH};

impl ObstacleContainer for CarSwitchLane {
    fn start_connecting_overlaps(&mut self, other: &CVec<ObstacleContainerID>, world: &mut World) {
        // Does nothing, normal lanes start connecting
        // TODO: verify this
    }

    fn connect_overlaps(
        &mut self,
        other_obstacle_container: ObstacleContainerID,
        other_path: &LinePath,
        other_width: N,
        reply_needed: bool,
        world: &mut World,
    ) {
        // make sure that other obstacle container is a car lane
        // TODO: we really need a way to check actor IDs for type at runtime
        if other_obstacle_container.as_raw().type_id
            != CarLaneID::local_broadcast(world).as_raw().type_id
        {
            return;
        };
        let other_car_lane = CarLaneID::from_raw(other_obstacle_container.as_raw());

        let projections = (
            other_path.project_with_max_distance(
                self.construction.path.start(),
                MAX_SWITCHING_LANE_DISTANCE,
                MAX_SWITCHING_LANE_DISTANCE,
            ),
            other_path.project_with_max_distance(
                self.construction.path.end(),
                MAX_SWITCHING_LANE_DISTANCE,
                MAX_SWITCHING_LANE_DISTANCE,
            ),
        );
        if let (
            Some((start_on_other_distance, start_on_other)),
            Some((end_on_other_distance, end_on_other)),
        ) = projections
        {
            if start_on_other_distance < end_on_other_distance
                && end_on_other_distance - start_on_other_distance > MIN_SWITCHING_LANE_LENGTH
                && start_on_other
                    .rough_eq_by(self.construction.path.start(), MAX_SWITCHING_LANE_DISTANCE)
                && end_on_other.rough_eq_by(self.construction.path.end(), 3.0)
            {
                let mut distance_covered = 0.0;
                let distance_map = self
                    .construction
                    .path
                    .segments()
                    .map(|segment| {
                        distance_covered += segment.length();
                        let segment_end_on_other_distance = other_path
                            .project_with_tolerance(segment.end(), MAX_SWITCHING_LANE_DISTANCE)
                            .expect("should contain switch lane segment end")
                            .0;
                        (
                            distance_covered,
                            segment_end_on_other_distance - start_on_other_distance,
                        )
                    })
                    .collect();

                let other_is_right = (start_on_other - self.construction.path.start())
                    .dot(&self.construction.path.start_direction().orthogonal_right())
                    > 0.0;

                if other_is_right {
                    self.connectivity.right = Some((
                        other_car_lane,
                        start_on_other_distance,
                        end_on_other_distance,
                    ));
                    self.connectivity.right_distance_map = distance_map;
                } else {
                    self.connectivity.left = Some((
                        other_car_lane,
                        start_on_other_distance,
                        end_on_other_distance,
                    ));
                    self.connectivity.left_distance_map = distance_map;
                }

                if let (
                    Some((left, left_start_on_other_distance, left_end_on_other_distance)),
                    Some((right, right_start_on_other_distance, right_end_on_other_distance)),
                ) = (self.connectivity.left, self.connectivity.right)
                {
                    left.add_switch_lane_interaction(
                        CarLaneInteraction::Switch {
                            via: self.id,
                            to: right,
                            start: left_start_on_other_distance,
                            end: left_end_on_other_distance,
                            is_left: false,
                        },
                        world,
                    );

                    right.add_switch_lane_interaction(
                        CarLaneInteraction::Switch {
                            via: self.id,
                            to: left,
                            start: right_start_on_other_distance,
                            end: right_end_on_other_distance,
                            is_left: true,
                        },
                        world,
                    );
                }
            }
        }
    }

    fn add_obstacles(
        &mut self,
        obstacles: &CVec<Obstacle>,
        from: ObstacleContainerID,
        world: &mut World,
    ) {
        if let (Some((left_id, ..)), Some(_)) = (self.connectivity.left, self.connectivity.right) {
            // TODO: ugly: untyped RawID shenanigans
            if left_id.as_raw() == from.as_raw() {
                self.microtraffic.left_obstacles = obstacles
                    .iter()
                    .map(|obstacle| {
                        obstacle
                            .offset_by(self.interaction_to_self_offset(*obstacle.position, true))
                    })
                    .collect();
            } else {
                self.microtraffic.right_obstacles = obstacles
                    .iter()
                    .map(|obstacle| {
                        obstacle
                            .offset_by(self.interaction_to_self_offset(*obstacle.position, false))
                    })
                    .collect();
            };
        } else {
            debug(
                LOG_T,
                "switch lane not connected for obstacles yet",
                self.id(),
                world,
            );
        }
    }

    fn disconnect(&mut self, other: ObstacleContainerID, world: &mut World) {
        self.connectivity.left = self
            .connectivity
            .left
            .filter(|(left_id, ..)| Into::<ObstacleContainerID>::into(*left_id) != other);
        self.connectivity.right = self
            .connectivity
            .right
            .filter(|(right_id, ..)| Into::<ObstacleContainerID>::into(*right_id) != other);
        other.on_confirm_disconnect(world);
    }

    fn on_confirm_disconnect(&mut self, world: &mut World) -> Fate {
        self.construction.disconnects_remaining -= 1;
        if self.construction.disconnects_remaining == 0 {
            self.finalize(
                self.construction
                    .unbuilding_for
                    .expect("should be unbuilding"),
                world,
            );
            Fate::Die
        } else {
            Fate::Live
        }
    }
}

impl Temporal for CarSwitchLane {
    fn tick(&mut self, dt: f32, current_instant: Instant, world: &mut World) {
        let dt = dt / MICROTRAFFIC_UNREALISTIC_SLOWDOWN;

        // self.construction.progress += dt * 400.0;

        let do_traffic = current_instant.ticks() % TRAFFIC_LOGIC_THROTTLING
            == self.id.as_raw().instance_id as usize % TRAFFIC_LOGIC_THROTTLING;

        if do_traffic {
            // TODO: optimize using BinaryHeap?
            self.microtraffic
                .left_obstacles
                .sort_by_key(|obstacle| obstacle.position);
            self.microtraffic
                .right_obstacles
                .sort_by_key(|obstacle| obstacle.position);

            for c in 0..self.microtraffic.cars.len() {
                let (acceleration, dangerous) = {
                    let car = &self.microtraffic.cars[c];
                    let next_car = self
                        .microtraffic
                        .cars
                        .iter()
                        .find(|other_car| *other_car.position > *car.position)
                        .map(|other_car| &other_car.as_obstacle);

                    let maybe_next_left_obstacle =
                        if car.switch_position < 0.3 || car.switch_acceleration < 0.0 {
                            self.microtraffic
                                .left_obstacles
                                .iter()
                                .find(|obstacle| *obstacle.position + 5.0 > *car.position)
                        } else {
                            None
                        };

                    let maybe_next_right_obstacle =
                        if car.switch_position > -0.3 || car.switch_acceleration > 0.0 {
                            self.microtraffic
                                .right_obstacles
                                .iter()
                                .find(|obstacle| *obstacle.position + 5.0 > *car.position)
                        } else {
                            None
                        };

                    let mut dangerous = false;
                    let next_obstacle_acceleration = *next_car
                        .into_iter()
                        .chain(maybe_next_left_obstacle)
                        .chain(maybe_next_right_obstacle)
                        .chain(&[Obstacle::far_ahead()])
                        .filter_map(|obstacle| {
                            if *obstacle.position < *car.position + 0.1 {
                                dangerous = true;
                                None
                            } else {
                                Some(OrderedFloat(intelligent_acceleration(car, obstacle, 1.0)))
                            }
                        })
                        .min()
                        .unwrap();

                    let switch_before_end_velocity =
                        (self.construction.length + 1.0 - *car.position) / 1.5;
                    let switch_before_end_acceleration = switch_before_end_velocity - car.velocity;

                    (
                        next_obstacle_acceleration.min(switch_before_end_acceleration),
                        dangerous,
                    )
                };

                let car = &mut self.microtraffic.cars[c];
                car.acceleration = acceleration;

                if dangerous && !car.cancelling {
                    car.switch_acceleration = -car.switch_acceleration;
                    car.cancelling = true;
                }
            }
        }

        for car in &mut self.microtraffic.cars {
            *car.position += dt * car.velocity;
            car.velocity = (car.velocity + dt * car.acceleration)
                .min(car.max_velocity)
                .max(0.0);
            car.switch_position += dt * car.switch_velocity;
            car.switch_velocity += dt * car.switch_acceleration;
            if car.switch_velocity.abs() > car.velocity / 12.0 {
                car.switch_velocity = car.velocity / 12.0 * car.switch_velocity.signum();
            }
        }

        for obstacle in self
            .microtraffic
            .left_obstacles
            .iter_mut()
            .chain(self.microtraffic.right_obstacles.iter_mut())
        {
            *obstacle.position += dt * obstacle.velocity;
        }

        if self.microtraffic.cars.len() > 1 {
            for i in (0..self.microtraffic.cars.len() - 1).rev() {
                if self.microtraffic.cars[i].position > self.microtraffic.cars[i + 1].position {
                    self.microtraffic.cars.swap(i, i + 1);
                }
            }
        }

        if let (Some((left, left_start, _)), Some((right, right_start, _))) =
            (self.connectivity.left, self.connectivity.right)
        {
            let mut i = 0;
            loop {
                let (should_remove, done) = if let Some(car) = self.microtraffic.cars.get(i) {
                    if car.switch_position > 1.0
                        || (*car.position > self.construction.length
                            && car.switch_acceleration > 0.0)
                    {
                        let right_as_lane: CarLaneLikeID = right.into();
                        right_as_lane.add_car(
                            car.as_lane_car.offset_by(
                                right_start + self.self_to_interaction_offset(*car.position, false),
                            ),
                            Some(self.id_as()),
                            current_instant,
                            world,
                        );
                        (true, false)
                    } else if car.switch_position < -1.0
                        || (*car.position > self.construction.length
                            && car.switch_acceleration <= 0.0)
                    {
                        let left_as_lane: CarLaneLikeID = left.into();
                        left_as_lane.add_car(
                            car.as_lane_car.offset_by(
                                left_start + self.self_to_interaction_offset(*car.position, true),
                            ),
                            Some(self.id_as()),
                            current_instant,
                            world,
                        );
                        (true, false)
                    } else {
                        i += 1;
                        (false, false)
                    }
                } else {
                    (false, true)
                };
                if done {
                    break;
                }
                if should_remove {
                    self.microtraffic.cars.remove(i);
                }
            }

            if (current_instant.ticks() + 1) % TRAFFIC_LOGIC_THROTTLING
                == left.as_raw().instance_id as usize % TRAFFIC_LOGIC_THROTTLING
            {
                let obstacles = self
                    .microtraffic
                    .cars
                    .iter()
                    .filter_map(|car| {
                        if car.switch_position < 0.3 || car.switch_acceleration < 0.0 {
                            Some(car.as_obstacle.offset_by(
                                left_start + self.self_to_interaction_offset(*car.position, true),
                            ))
                        } else {
                            None
                        }
                    })
                    .collect();
                Into::<ObstacleContainerID>::into(left).add_obstacles(
                    obstacles,
                    self.id_as(),
                    world,
                );
            }

            if (current_instant.ticks() + 1) % TRAFFIC_LOGIC_THROTTLING
                == right.as_raw().instance_id as usize % TRAFFIC_LOGIC_THROTTLING
            {
                let obstacles = self
                    .microtraffic
                    .cars
                    .iter()
                    .filter_map(|car| {
                        if car.switch_position > -0.3 || car.switch_acceleration > 0.0 {
                            Some(car.as_obstacle.offset_by(
                                right_start + self.self_to_interaction_offset(*car.position, false),
                            ))
                        } else {
                            None
                        }
                    })
                    .collect();
                Into::<ObstacleContainerID>::into(right).add_obstacles(
                    obstacles,
                    self.id_as(),
                    world,
                );
            }
        }
    }
}

pub fn setup(system: &mut ActorSystem) {
    auto_setup(system);
}

fn obstacles_for_interaction(
    interaction: &CarLaneInteraction,
    mut cars: ::std::slice::Iter<LaneCar>,
    self_obstacles_iter: ::std::slice::Iter<(Obstacle, ObstacleContainerID)>,
) -> Option<CVec<Obstacle>> {
    match *interaction {
        CarLaneInteraction::Conflicting {
            start,
            conflicting_start,
            end,
            can_weave,
            ..
        } => {
            if can_weave {
                Some(
                    cars.skip_while(|car: &&LaneCar| *car.position + 2.0 * car.velocity < start)
                        .take_while(|car: &&LaneCar| *car.position < end)
                        .map(|car| car.as_obstacle.offset_by(-start + conflicting_start))
                        .collect(),
                )
            } else {
                let in_overlap = |car: &LaneCar| {
                    *car.position + 2.0 * car.velocity > start && *car.position - 2.0 < end
                };
                if cars.any(in_overlap) {
                    Some(
                        vec![Obstacle {
                            position: OrderedFloat(conflicting_start),
                            velocity: 0.0,
                            max_velocity: 0.0,
                        }]
                        .into(),
                    )
                } else {
                    Some(CVec::new())
                }
            }
        }
        CarLaneInteraction::Switch { start, end, .. } => Some(
            cars.skip_while(|car: &&LaneCar| *car.position + 2.0 * car.velocity < start)
                .take_while(|car: &&LaneCar| *car.position < end)
                .map(|car| car.as_obstacle)
                .collect(),
        ),
        CarLaneInteraction::Previous {
            previous_length, ..
        } => Some(
            cars.map(|car| &car.as_obstacle)
                .chain(self_obstacles_iter.map(|&(ref obstacle, _id)| obstacle))
                .find(|car| *car.position >= -2.0)
                .map(|first_car| first_car.offset_by(previous_length))
                .into_iter()
                .collect(),
        ),
        _ => None,
    }
}

fn obstacles_for_sidewalk_interaction(
    interaction: &SidewalkInteraction,
    mut pedestrians: ::std::slice::Iter<Pedestrian>,
    self_obstacles_iter: ::std::slice::Iter<(Obstacle, ObstacleContainerID)>,
) -> Option<CVec<Obstacle>> {
    match *interaction {
        SidewalkInteraction::Conflicting {
            start,
            conflicting_start,
            end,
            ..
        } => {
            let in_overlap = |pedestrian: &Pedestrian| {
                *pedestrian.position + 2.0 * pedestrian.velocity > start
                    && *pedestrian.position - 2.0 < end
            };
            if pedestrians.any(in_overlap) {
                Some(
                    vec![Obstacle {
                        position: OrderedFloat(conflicting_start),
                        velocity: 0.0,
                        max_velocity: 0.0,
                    }]
                    .into(),
                )
            } else {
                Some(CVec::new())
            }
        }
        _ => None,
    }
}

#[derive(Compact, Clone)]
pub struct SidewalkMicrotraffic {
    pub obstacles: CVec<(Obstacle, ObstacleContainerID)>,
    pub pedestrians: CVec<Pedestrian>,
    timings: CVec<bool>,
    pub green: bool,
}

impl SidewalkMicrotraffic {
    pub fn new(timings: CVec<bool>) -> Self {
        SidewalkMicrotraffic {
            obstacles: CVec::new(),
            pedestrians: CVec::new(),
            timings,
            green: false,
        }
    }
}

#[derive(Copy, Clone)]
pub struct Pedestrian {
    pub trip: TripID,
    pub as_obstacle: Obstacle,
    pub destination: pathfinding::PreciseLocation,
    pub next_hop_interaction: Option<u8>,
}

impl Deref for Pedestrian {
    type Target = Obstacle;

    fn deref(&self) -> &Obstacle {
        &self.as_obstacle
    }
}

impl DerefMut for Pedestrian {
    fn deref_mut(&mut self) -> &mut Obstacle {
        &mut self.as_obstacle
    }
}

impl Sidewalk {
    pub fn add_pedestrian(&mut self, pedestrian: Pedestrian, instant: Instant, world: &mut World) {
        // TODO: does this really happen, or is it ok to only hacke this logic in tick()?
        if let Some(self_as_location) = self.pathfinding.location {
            if pedestrian.destination.location == self_as_location
                && *pedestrian.position >= pedestrian.destination.offset
            {
                pedestrian.trip.finish(
                    TripResult {
                        location_now: None,
                        fate: TripFate::Success(instant),
                    },
                    world,
                );

                return;
            }
        }

        let (maybe_next_hop_interaction, almost_there) =
            if Some(pedestrian.destination.location) == self.pathfinding.location {
                (None, true)
            } else {
                let maybe_hop = self
                    .pathfinding
                    .routes
                    .get(pedestrian.destination.location)
                    .or_else(|| {
                        self.pathfinding
                            .routes
                            .get(pedestrian.destination.landmark_destination())
                    })
                    .map(|&StoredRoutingEntry { outgoing_idx, .. }| outgoing_idx as usize);

                (maybe_hop, false)
            };

        if maybe_next_hop_interaction.is_some() || almost_there {
            let routed_pedestrian = Pedestrian {
                next_hop_interaction: maybe_next_hop_interaction.map(|hop| hop as u8),
                ..pedestrian
            };

            self.microtraffic.pedestrians.push(routed_pedestrian);
        } else {
            pedestrian.trip.finish(
                TripResult {
                    location_now: Some(self.id_as()),
                    fate: TripFate::NoRoute,
                },
                world,
            );
        }
    }
}

impl ObstacleContainer for Sidewalk {
    fn start_connecting_overlaps(
        &mut self,
        other_obstacle_containers: &CVec<ObstacleContainerID>,
        world: &mut World,
    ) {
        for &other_obstacle_container in other_obstacle_containers.iter() {
            other_obstacle_container.connect_overlaps(
                self.id_as(),
                self.construction.path.clone(),
                LANE_WIDTH_AS_OBSTACLE_CONTAINER,
                true,
                world,
            );
        }
    }

    fn connect_overlaps(
        &mut self,
        other_id: ObstacleContainerID,
        other_path: &LinePath,
        other_width: N,
        reply_needed: bool,
        world: &mut World,
    ) {
        // make sure that other obstacle container is a car lane
        // TODO: we really need a way to check actor IDs for type at runtime
        if other_id.as_raw().type_id != CarLaneID::local_broadcast(world).as_raw().type_id {
            return;
        };

        let other_car_lane = CarLaneID::from_raw(other_id.as_raw());

        let &(ref lane_band, ref lane_outline) = {
            let band = Band::new(
                self.construction.path.clone(),
                LANE_WIDTH_AS_OBSTACLE_CONTAINER,
            );
            let outline = band.outline();
            &(band, outline)
        };

        let &(ref other_band, ref other_outline) = {
            let band = Band::new(other_path.clone(), LANE_WIDTH_AS_OBSTACLE_CONTAINER);
            let outline = band.outline();
            &(band, outline)
        };

        let intersections = (lane_outline, other_outline).intersect();
        if intersections.len() >= 2 {
            if let ::itertools::MinMaxResult::MinMax(
                (entry_intersection, entry_distance),
                (exit_intersection, exit_distance),
            ) = intersections
                .iter()
                .map(|intersection| {
                    (
                        intersection,
                        lane_band.outline_distance_to_path_distance(intersection.along_a),
                    )
                })
                .minmax_by_key(|&(_, distance)| OrderedFloat(distance))
            {
                let other_entry_distance =
                    other_band.outline_distance_to_path_distance(entry_intersection.along_b);
                let other_exit_distance =
                    other_band.outline_distance_to_path_distance(exit_intersection.along_b);

                self.connectivity
                    .interactions
                    .push(SidewalkInteraction::Conflicting {
                        conflicting: other_car_lane,
                        start: entry_distance,
                        conflicting_start: other_entry_distance.min(other_exit_distance),
                        end: exit_distance,
                        conflicting_end: other_exit_distance.max(other_entry_distance),
                    });
            } else {
                panic!("both entry and exit should exist")
            }
        }

        if reply_needed {
            other_id.connect_overlaps(
                self.id_as(),
                self.construction.path.clone(),
                LANE_WIDTH_AS_OBSTACLE_CONTAINER,
                false,
                world,
            );
        }
    }

    fn add_obstacles(
        &mut self,
        obstacles: &CVec<Obstacle>,
        from: ObstacleContainerID,
        _: &mut World,
    ) {
        self.microtraffic
            .obstacles
            .retain(|&(_, received_from)| received_from != from);
        self.microtraffic
            .obstacles
            .extend(obstacles.iter().map(|obstacle| (*obstacle, from)));
    }

    fn disconnect(&mut self, other_id: ObstacleContainerID, world: &mut World) {
        self.connectivity
            .interactions
            .retain(|interaction| match interaction {
                SidewalkInteraction::Conflicting { conflicting, .. } => {
                    conflicting.into() != other_id
                }
                SidewalkInteraction::Next { next, .. } => next.into() != other_id,
                SidewalkInteraction::Previous { previous, .. } => previous.into() != other_id,
            });

        self.microtraffic.obstacles.drain();

        let self_as_rough_location = self.id_as();

        for pedestrian in self.microtraffic.pedestrians.drain() {
            pedestrian.trip.finish(
                TripResult {
                    location_now: Some(self_as_rough_location),
                    fate: TripFate::HopDisconnected,
                },
                world,
            );
        }

        ::transport::pathfinding::Link::on_disconnect(self);
        other_id.on_confirm_disconnect(world);
    }

    fn on_confirm_disconnect(&mut self, world: &mut World) -> Fate {
        self.construction.disconnects_remaining -= 1;
        if self.construction.disconnects_remaining == 0 {
            self.finalize(
                self.construction
                    .unbuilding_for
                    .expect("should be unbuilding"),
                world,
            );
            Fate::Die
        } else {
            Fate::Live
        }
    }
}

const PEDESTRIAN_STOP_DISTANCE: N = 2.0;

impl Temporal for Sidewalk {
    fn tick(&mut self, dt: f32, current_instant: Instant, world: &mut World) {
        let dt = dt / MICROTRAFFIC_UNREALISTIC_SLOWDOWN;

        let do_traffic = current_instant.ticks() % TRAFFIC_LOGIC_THROTTLING
            == self.id.as_raw().instance_id as usize % TRAFFIC_LOGIC_THROTTLING;

        self.microtraffic.green = if self.microtraffic.timings.is_empty() {
            true
        } else {
            self.microtraffic.timings
                [(current_instant.ticks() / 30) % self.microtraffic.timings.len()]
        };

        if current_instant.ticks() % PATHFINDING_THROTTLING
            == self.id.as_raw().instance_id as usize % PATHFINDING_THROTTLING
        {
            self.pathfinding_tick(world);
        }

        if do_traffic {
            for &mut pedestrian in self.microtraffic.pedestrians.iter_mut() {
                let close_obstacle = self.microtraffic.obstacles.iter().any(|&(obstacle, _)| {
                    obstacle.position > pedestrian.position
                        && (*obstacle.position - *pedestrian.position < PEDESTRIAN_STOP_DISTANCE)
                });

                if close_obstacle || !self.microtraffic.green {
                    pedestrian.velocity = 0.0;
                } else {
                    pedestrian.velocity = pedestrian.max_velocity;
                }
            }

            for &mut pedestrian in self.microtraffic.pedestrians.iter_mut() {
                *pedestrian.position += dt * pedestrian.velocity;
            }

            if let Some(self_as_location) = self.pathfinding.location {
                self.microtraffic.pedestrians.retain(|pedestrian| {
                    if pedestrian.destination.location == self_as_location
                        && *pedestrian.position >= pedestrian.destination.offset
                    {
                        pedestrian.trip.finish(
                            TripResult {
                                location_now: None,
                                fate: TripFate::Success(current_instant),
                            },
                            world,
                        );

                        false
                    } else {
                        true
                    }
                });
            }

            loop {
                let maybe_switch_pedestrian: Option<(usize, SidewalkID, f32)> = self
                    .microtraffic
                    .pedestrians
                    .iter()
                    .enumerate()
                    .rev()
                    .filter_map(|(i, &pedestrian)| {
                        let interaction = pedestrian.next_hop_interaction.map(|hop_interaction| {
                            self.connectivity.interactions[hop_interaction as usize]
                        });

                        match interaction {
                            Some(SidewalkInteraction::Next { next, .. }) => {
                                if *pedestrian.position > self.construction.length {
                                    Some((i, next.into(), self.construction.length))
                                } else {
                                    None
                                }
                            }
                            Some(_) => unreachable!(
                                "Pedestrian has a next hop that is not a Next sidewalk"
                            ),
                            None => None,
                        }
                    })
                    .next();

                if let Some((idx_to_remove, next_sidewalk, start)) = maybe_switch_pedestrian {
                    let pedestrian = self.microtraffic.pedestrians.remove(idx_to_remove);
                    next_sidewalk.add_pedestrian(
                        Pedestrian {
                            as_obstacle: pedestrian.offset_by(-start),
                            ..pedestrian
                        },
                        current_instant,
                        world,
                    );
                } else {
                    break;
                }

                // ASSUMPTION: only one interaction per Lane/Lane pair
                for interaction in self.connectivity.interactions.iter() {
                    let pedestrians = self.microtraffic.pedestrians.iter();

                    match interaction {
                        SidewalkInteraction::Conflicting { conflicting, .. } => {
                            if (current_instant.ticks() + 1) % TRAFFIC_LOGIC_THROTTLING
                                == conflicting.as_raw().instance_id as usize
                                    % TRAFFIC_LOGIC_THROTTLING
                            {
                                let maybe_obstacles = obstacles_for_sidewalk_interaction(
                                    interaction,
                                    pedestrians,
                                    self.microtraffic.obstacles.iter(),
                                );

                                if let Some(obstacles) = maybe_obstacles {
                                    Into::<ObstacleContainerID>::into(*conflicting).add_obstacles(
                                        obstacles,
                                        self.id_as(),
                                        world,
                                    );
                                }
                            }
                        }
                        _ => {} // Next or prev don't create obstacles
                    }
                }
            }
        }
    }
}

mod kay_auto;
pub use self::kay_auto::*;
