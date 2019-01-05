use kay::{ActorSystem, World, TypedID, Actor};
use compact::CVec;
use ordered_float::OrderedFloat;
use std::f32::INFINITY;
use std::ops::{Deref, DerefMut};

use super::lane::{Lane, LaneID, SwitchLane, SwitchLaneID};
use super::lane::connectivity::{Interaction};
use super::pathfinding;

mod intelligent_acceleration;
use self::intelligent_acceleration::intelligent_acceleration;

use log::debug;
const LOG_T: &str = "Microtraffic";

// TODO: move all iteration, updates, etc into one huge retain loop (see identical TODO below)

#[derive(Compact, Clone)]
pub struct Microtraffic {
    pub obstacles: CVec<(Obstacle, LaneLikeID)>,
    pub cars: CVec<LaneCar>,
    timings: CVec<bool>,
    pub green: bool,
    pub yellow_to_green: bool,
    pub yellow_to_red: bool,
}

impl Microtraffic {
    pub fn new(timings: CVec<bool>) -> Self {
        Microtraffic {
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
pub struct TransferringMicrotraffic {
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
use super::pathfinding::Node;

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

use time::Instant;

pub trait LaneLike {
    fn add_car(
        &mut self,
        car: LaneCar,
        from: Option<LaneLikeID>,
        instant: Instant,
        world: &mut World,
    );
    fn add_obstacles(&mut self, obstacles: &CVec<Obstacle>, from: LaneLikeID, world: &mut World);
}

use self::pathfinding::RoutingInfo;

use time::{Temporal, TemporalID};

const TRAFFIC_LOGIC_THROTTLING: usize = 10;
const PATHFINDING_THROTTLING: usize = 10;

impl LaneLike for Lane {
    fn add_car(
        &mut self,
        car: LaneCar,
        _from: Option<LaneLikeID>,
        instant: Instant,
        world: &mut World,
    ) {
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
                    .map(|&RoutingInfo { outgoing_idx, .. }| outgoing_idx as usize);

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

    fn add_obstacles(&mut self, obstacles: &CVec<Obstacle>, from: LaneLikeID, _: &mut World) {
        self.microtraffic
            .obstacles
            .retain(|&(_, received_from)| received_from != from);
        self.microtraffic
            .obstacles
            .extend(obstacles.iter().map(|obstacle| (*obstacle, from)));
    }
}

impl Lane {
    pub fn on_signal_changed(&mut self, from: LaneID, new_green: bool, _: &mut World) {
        for interaction in self.connectivity.interactions.iter_mut() {
            match *interaction {
                Interaction::Next {
                    next,
                    ref mut green,
                } if next == from => *green = new_green,
                _ => {}
            }
        }
    }
}

impl Temporal for Lane {
    fn tick(&mut self, dt: f32, current_instant: Instant, world: &mut World) {
        let dt = dt / MICROTRAFFIC_UNREALISTIC_SLOWDOWN;

        self.construction.progress += dt * 400.0;

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
                if let Interaction::Previous { previous, .. } = *interaction {
                    previous.on_signal_changed(self.id, self.microtraffic.green, world);
                }
            }
        }

        if current_instant.ticks() % PATHFINDING_THROTTLING
            == self.id.as_raw().instance_id as usize % PATHFINDING_THROTTLING
        {
            self.update_routes(world);
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
                    if let Interaction::Next { green, .. } =
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
            let maybe_switch_car: Option<(usize, LaneLikeID, f32)> = self
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
                        Some(Interaction::Switch {
                            start, end, via, ..
                        }) => {
                            if *car.position > start && *car.position > end - 300.0 {
                                Some((i, via.into(), start))
                            } else {
                                None
                            }
                        }
                        Some(Interaction::Next { next, .. }) => {
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

impl LaneLike for SwitchLane {
    fn add_car(
        &mut self,
        car: LaneCar,
        maybe_from: Option<LaneLikeID>,
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

    fn add_obstacles(&mut self, obstacles: &CVec<Obstacle>, from: LaneLikeID, world: &mut World) {
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
}

impl Temporal for SwitchLane {
    fn tick(&mut self, dt: f32, current_instant: Instant, world: &mut World) {
        let dt = dt / MICROTRAFFIC_UNREALISTIC_SLOWDOWN;

        self.construction.progress += dt * 400.0;

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
                        let right_as_lane: LaneLikeID = right.into();
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
                        let left_as_lane: LaneLikeID = left.into();
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
                let left_as_lane: LaneLikeID = left.into();
                left_as_lane.add_obstacles(obstacles, self.id_as(), world);
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
                let right_as_lane: LaneLikeID = right.into();
                right_as_lane.add_obstacles(obstacles, self.id_as(), world);
            }
        }
    }
}

pub fn setup(system: &mut ActorSystem) {
    auto_setup(system);
}

fn obstacles_for_interaction(
    interaction: &Interaction,
    mut cars: ::std::slice::Iter<LaneCar>,
    self_obstacles_iter: ::std::slice::Iter<(Obstacle, LaneLikeID)>,
) -> Option<CVec<Obstacle>> {
    match *interaction {
        Interaction::Conflicting {
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
        Interaction::Switch { start, end, .. } => Some(
            cars.skip_while(|car: &&LaneCar| *car.position + 2.0 * car.velocity < start)
                .take_while(|car: &&LaneCar| *car.position < end)
                .map(|car| car.as_obstacle)
                .collect(),
        ),
        Interaction::Previous {
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

mod kay_auto;
pub use self::kay_auto::*;
