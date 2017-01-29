use kay::{ID, Recipient, Actor, Fate};
use kay::swarm::{Swarm, SubActor};
use compact::CVec;
use ordered_float::OrderedFloat;
use ::std::f32::INFINITY;
use ::std::ops::{Deref, DerefMut};

use super::lane::{Lane, TransferLane};
use super::connectivity::{Interaction, InteractionKind, OverlapKind};
use super::pathfinding;

mod intelligent_acceleration;
use self::intelligent_acceleration::intelligent_acceleration;

#[derive(Compact, Clone)]
pub struct Microtraffic {
    pub obstacles: CVec<(Obstacle, ID)>,
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
            timings: timings,
            green: false,
            yellow_to_green: false,
            yellow_to_red: false,
        }
    }
}

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
    fn far_behind() -> Obstacle {
        Obstacle {
            position: OrderedFloat(-INFINITY),
            velocity: 0.0,
            max_velocity: 20.0,
        }
    }
    fn offset_by(&self, delta: f32) -> Obstacle {
        Obstacle { position: OrderedFloat(*self.position + delta), ..*self }
    }
}

#[derive(Copy, Clone)]
pub struct LaneCar {
    pub trip: ID,
    pub as_obstacle: Obstacle,
    pub acceleration: f32,
    pub destination: pathfinding::Destination,
    pub next_hop_interaction: u8,
}

impl LaneCar {
    fn offset_by(&self, delta: f32) -> LaneCar {
        LaneCar { as_obstacle: self.as_obstacle.offset_by(delta), ..*self }
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
    pub transfer_position: f32,
    pub transfer_velocity: f32,
    pub transfer_acceleration: f32,
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

#[derive(Copy, Clone)]
pub struct AddCar {
    pub car: LaneCar,
    pub from: Option<ID>,
}

#[derive(Compact, Clone)]
struct AddObstacles {
    obstacles: CVec<Obstacle>,
    from: ID,
}

use self::pathfinding::RoutingInfo;

impl Recipient<AddCar> for Lane {
    fn receive(&mut self, msg: &AddCar) -> Fate {
        match *msg {
            AddCar { car, .. } => {
                // TODO: horrible hack to encode it like this
                let car_forcibly_spawned = *car.as_obstacle.position < 0.0;

                let maybe_next_hop_interaction = self.pathfinding
                    .routes
                    .get(car.destination)
                    .or(self.pathfinding
                        .routes
                        .get(car.destination.landmark_destination()))
                    .or_else(|| {
                        println!("NO ROUTE!");
                        if car_forcibly_spawned || self.pathfinding.routes.is_empty() {
                            None
                        } else {
                            // pseudorandom, lol
                            self.pathfinding
                                .routes
                                .values()
                                .nth((car.velocity * 10000.0) as usize %
                                     self.pathfinding.routes.len())
                        }
                    })
                    .map(|&RoutingInfo { outgoing_idx, .. }| outgoing_idx as usize);

                let spawn_possible = if car_forcibly_spawned {
                    if self.last_spawn_position > 2.0 {
                        Some(true)
                    } else {
                        None
                    }
                } else {
                    Some(true)
                };

                if let (Some(next_hop_interaction), Some(_)) =
                    (maybe_next_hop_interaction, spawn_possible) {
                    let routed_car = LaneCar {
                        next_hop_interaction: next_hop_interaction as u8,
                        as_obstacle: if car_forcibly_spawned {
                            self.last_spawn_position -= 6.0;
                            car.as_obstacle
                                .offset_by(-*car.as_obstacle.position)
                                .offset_by(self.last_spawn_position + 6.0)
                        } else {
                            car.as_obstacle
                        },
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
                    // TODO: cancel trip
                }

                Fate::Live
            }
        }
    }
}

impl Recipient<AddCar> for TransferLane {
    fn receive(&mut self, msg: &AddCar) -> Fate {
        match *msg {
            AddCar { car, from: Some(from) } => {
                let from_left = from == self.connectivity.left.expect("should have a left lane").0;
                let side_multiplier = if from_left { -1.0 } else { 1.0 };
                let offset = self.interaction_to_self_offset(*car.position, from_left);
                self.microtraffic.cars.push(TransferringLaneCar {
                    as_lane_car: car.offset_by(offset),
                    transfer_position: 1.0 * side_multiplier,
                    transfer_velocity: 0.0,
                    transfer_acceleration: 0.3 * -side_multiplier,
                    cancelling: false,
                });
                // TODO: optimize using BinaryHeap?
                self.microtraffic.cars.sort_by_key(|car| car.as_obstacle.position);
                Fate::Live
            }
            AddCar { from: None, .. } => {
                panic!("car has to come from somewhere on a transfer lane")
            }
        }
    }
}

impl Recipient<AddObstacles> for Lane {
    fn receive(&mut self, msg: &AddObstacles) -> Fate {
        match *msg {
            AddObstacles { ref obstacles, from } => {
                self.microtraffic.obstacles.retain(|&(_, received_from)| received_from != from);
                self.microtraffic
                    .obstacles
                    .extend(obstacles.iter().map(|obstacle| (*obstacle, from)));
                Fate::Live
            }
        }
    }
}

impl Recipient<AddObstacles> for TransferLane {
    fn receive(&mut self, msg: &AddObstacles) -> Fate {
        match *msg {
            AddObstacles { ref obstacles, from } => {
                if let (Some((left_id, _)), Some(_)) =
                    (self.connectivity.left, self.connectivity.right) {
                    if left_id == from {
                        self.microtraffic.left_obstacles = obstacles.iter()
                            .map(|obstacle| {
                                obstacle.offset_by(
                                    self.interaction_to_self_offset(*obstacle.position, true)
                                )
                            })
                            .collect();
                    } else {
                        self.microtraffic.right_obstacles = obstacles.iter()
                            .map(|obstacle| {
                                obstacle.offset_by(
                                    self.interaction_to_self_offset(*obstacle.position, false)
                                )
                            })
                            .collect();
                    };
                } else {
                    println!("transfer lane not connected for obstacles yet");
                }
                Fate::Live
            }
        }
    }
}

use core::simulation::Tick;

const TRAFFIC_LOGIC_THROTTLING: usize = 30;
const PATHFINDING_THROTTLING: usize = 10;

#[derive(Copy, Clone)]
pub struct SignalChanged {
    from: ID,
    green: bool,
}

impl Recipient<Tick> for Lane {
    fn receive(&mut self, msg: &Tick) -> Fate {
        match *msg {
            Tick { dt, current_tick } => {
                self.construction.progress += dt * 400.0;

                let do_traffic = current_tick % TRAFFIC_LOGIC_THROTTLING ==
                                 self.id().sub_actor_id as usize % TRAFFIC_LOGIC_THROTTLING;

                let old_green = self.microtraffic.green;
                self.microtraffic.yellow_to_red = if self.microtraffic.timings.is_empty() {
                    true
                } else {
                    !self.microtraffic.timings[((current_tick + 100) / 25) %
                     self.microtraffic.timings.len()]
                };
                self.microtraffic.yellow_to_green = if self.microtraffic.timings.is_empty() {
                    true
                } else {
                    self.microtraffic.timings[((current_tick + 100) / 25) %
                    self.microtraffic.timings.len()]
                };
                self.microtraffic.green = if self.microtraffic.timings.is_empty() {
                    true
                } else {
                    self.microtraffic.timings[(current_tick / 25) % self.microtraffic.timings.len()]
                };

                // TODO: this is just a hacky way to update new lanes about existing lane's green
                if old_green != self.microtraffic.green || do_traffic {
                    for interaction in &self.connectivity.interactions {
                        if let Interaction { kind: InteractionKind::Previous { .. },
                                             partner_lane,
                                             .. } = *interaction {
                            partner_lane <<
                            SignalChanged {
                                from: self.id(),
                                green: self.microtraffic.green,
                            }
                        }
                    }
                }

                if current_tick % PATHFINDING_THROTTLING ==
                   self.id().sub_actor_id as usize % PATHFINDING_THROTTLING {
                    self::pathfinding::tick(self);
                }

                if do_traffic {
                    // TODO: optimize using BinaryHeap?
                    self.microtraffic
                        .obstacles
                        .sort_by_key(|&(ref obstacle, _id)| obstacle.position);

                    let mut obstacles =
                        self.microtraffic.obstacles.iter().map(|&(ref obstacle, _id)| obstacle);
                    let mut maybe_next_obstacle = obstacles.next();

                    for c in 0..self.microtraffic.cars.len() {
                        let next_obstacle = self.microtraffic
                            .cars
                            .get(c + 1)
                            .map_or(Obstacle::far_ahead(), |car| car.as_obstacle);
                        let car = &mut self.microtraffic.cars[c];
                        let next_car_acceleration =
                            intelligent_acceleration(car, &next_obstacle, 2.0);

                        maybe_next_obstacle = maybe_next_obstacle.and_then(|obstacle| {
                            let mut following_obstacle = Some(obstacle);
                            while following_obstacle.is_some() &&
                                  *following_obstacle.unwrap().position < *car.position + 0.1 {
                                following_obstacle = obstacles.next();
                            }
                            following_obstacle
                        });

                        let next_obstacle_acceleration = if let Some(next_obstacle) =
                            maybe_next_obstacle {
                            intelligent_acceleration(car, next_obstacle, 4.0)
                        } else {
                            INFINITY
                        };

                        car.acceleration = next_car_acceleration.min(next_obstacle_acceleration);

                        if let Interaction { start, kind: InteractionKind::Next { green }, .. } =
                            self.connectivity.interactions[car.next_hop_interaction as usize] {
                            if !green {
                                car.acceleration = car.acceleration
                                    .min(intelligent_acceleration(car,
                                                                  &Obstacle {
                                                                      position: OrderedFloat(start +
                                                                                             2.0),
                                                                      velocity: 0.0,
                                                                      max_velocity: 0.0,
                                                                  },
                                                                  2.0))
                            }
                        }
                    }
                }

                for car in &mut self.microtraffic.cars {
                    *car.position += dt * car.velocity;
                    car.velocity =
                        (car.velocity + dt * car.acceleration).min(car.max_velocity).max(0.0);
                }

                for &mut (ref mut obstacle, _id) in &mut self.microtraffic.obstacles {
                    *obstacle.position += dt * obstacle.velocity;
                }

                if self.microtraffic.cars.len() > 1 {
                    for i in (0..self.microtraffic.cars.len() - 1).rev() {
                        self.microtraffic.cars[i].position =
                            OrderedFloat((*self.microtraffic.cars[i].position)
                                .min(*self.microtraffic.cars[i + 1].position));
                    }
                }

                loop {
                    let maybe_switch_car = self.microtraffic
                        .cars
                        .iter()
                        .enumerate()
                        .rev()
                        .filter_map(|(i, &car)| {
                            let interaction = self.connectivity
                                .interactions
                                                  [car.next_hop_interaction as usize];

                            match interaction.kind {
                                InteractionKind::Overlap { end,
                                                           kind: OverlapKind::Transfer,
                                                           .. } => {
                                    if *car.position > interaction.start &&
                                       *car.position > end - 300.0 {
                                        Some((i,
                                              interaction.partner_lane,
                                              interaction.start,
                                              interaction.partner_start))
                                    } else {
                                        None
                                    }
                                }
                                _ => {
                                    if *car.position > interaction.start {
                                        Some((i,
                                              interaction.partner_lane,
                                              interaction.start,
                                              interaction.partner_start))
                                    } else {
                                        None
                                    }
                                }
                            }
                        })
                        .next();

                    if let Some((idx_to_remove, next_lane, start, partner_start)) =
                        maybe_switch_car {
                        let car = self.microtraffic.cars.remove(idx_to_remove);
                        if self.id() != car.destination.node {
                            next_lane <<
                            AddCar {
                                car: car.offset_by(partner_start - start),
                                from: Some(self.id()),
                            };
                        }
                    } else {
                        break;
                    }
                }

                // ASSUMPTION: only one interaction per Lane/Lane pair
                for interaction in self.connectivity.interactions.iter() {
                    let cars = self.microtraffic.cars.iter();

                    if (current_tick + 1) % TRAFFIC_LOGIC_THROTTLING ==
                       interaction.partner_lane.sub_actor_id as usize % TRAFFIC_LOGIC_THROTTLING {
                        let maybe_obstacles =
                            obstacles_for_interaction(interaction,
                                                      cars,
                                                      self.microtraffic.obstacles.iter());

                        if let Some(obstacles) = maybe_obstacles {
                            interaction.partner_lane <<
                            AddObstacles {
                                obstacles: obstacles,
                                from: self.id(),
                            }
                        }
                    }
                }

                Fate::Live
            }
        }
    }
}

fn obstacles_for_interaction(interaction: &Interaction,
                             mut cars: ::std::slice::Iter<LaneCar>,
                             self_obstacles_iter: ::std::slice::Iter<(Obstacle, ID)>)
                             -> Option<CVec<Obstacle>> {
    match *interaction {
        Interaction { partner_lane,
                      start,
                      partner_start,
                      kind: InteractionKind::Overlap { end, kind, .. },
                      .. } => {
            Some(match kind {
                OverlapKind::Parallel => {
                    cars.skip_while(|car: &&LaneCar| *car.position + 2.0 * car.velocity < start)
                        .take_while(|car: &&LaneCar| *car.position < end)
                        .map(|car| {
                            car.as_obstacle
                                .offset_by(-start + partner_start)
                        })
                        .collect()
                }
                OverlapKind::Transfer => {
                    cars.skip_while(|car: &&LaneCar| *car.position + 2.0 * car.velocity < start)
                        .map(|car| car.as_obstacle.offset_by(-start + partner_start))
                        .chain(self_obstacles_iter
                            .filter_map(|&(obstacle, id)| if id != partner_lane &&
                                                             *obstacle.position +
                                                             2.0 * obstacle.velocity >
                                                             start {
                                Some(obstacle.offset_by(-start + partner_start))
                            } else {
                                None
                            }))
                        .collect()
                }
                OverlapKind::Conflicting => {
                    let in_overlap = |car: &LaneCar| {
                        *car.position + 2.0 * car.velocity > start && *car.position - 2.0 < end
                    };
                    if cars.any(in_overlap) {
                        vec![Obstacle {
                                 position: OrderedFloat(partner_start),
                                 velocity: 0.0,
                                 max_velocity: 0.0,
                             }]
                            .into()
                    } else {
                        CVec::new()
                    }
                }
            })
        }
        Interaction { start, partner_start, kind: InteractionKind::Previous, .. } => {
            Some(cars.map(|car| &car.as_obstacle)
                .chain(self_obstacles_iter.map(|&(ref obstacle, _id)| obstacle))
                .find(|car| *car.position >= start - 2.0)
                .map(|first_car| first_car.offset_by(-start + partner_start))
                .into_iter()
                .collect())
        }
        Interaction { kind: InteractionKind::Next { .. }, .. } => {
            None
            // TODO: for looking backwards for merging lanes?
        }
    }
}

impl Recipient<Tick> for TransferLane {
    fn receive(&mut self, msg: &Tick) -> Fate {
        match *msg {
            Tick { dt, current_tick } => {
                self.construction.progress += dt * 400.0;

                let do_traffic = current_tick % TRAFFIC_LOGIC_THROTTLING ==
                                 self.id().sub_actor_id as usize % TRAFFIC_LOGIC_THROTTLING;

                if do_traffic {
                    // TODO: optimize using BinaryHeap?
                    self.microtraffic.left_obstacles.sort_by_key(|obstacle| obstacle.position);
                    self.microtraffic.right_obstacles.sort_by_key(|obstacle| obstacle.position);

                    for c in 0..self.microtraffic.cars.len() {
                        let (acceleration, dangerous) = {
                            let car = &self.microtraffic.cars[c];
                            let next_car = self.microtraffic
                                .cars
                                .iter()
                                .find(|other_car| *other_car.position > *car.position)
                                .map(|other_car| &other_car.as_obstacle);

                            let maybe_next_left_obstacle = if car.transfer_position < 0.3 ||
                                                              car.transfer_acceleration < 0.0 {
                                self.microtraffic
                                    .left_obstacles
                                    .iter()
                                    .find(|obstacle| *obstacle.position + 5.0 > *car.position)
                            } else {
                                None
                            };

                            let maybe_next_right_obstacle = if car.transfer_position > -0.3 ||
                                                               car.transfer_acceleration > 0.0 {
                                self.microtraffic
                                    .right_obstacles
                                    .iter()
                                    .find(|obstacle| *obstacle.position + 5.0 > *car.position)
                            } else {
                                None
                            };

                            let mut dangerous = false;
                            let next_obstacle_acceleration = *next_car.into_iter()
                                .chain(maybe_next_left_obstacle)
                                .chain(maybe_next_right_obstacle)
                                .chain(&[Obstacle::far_ahead()])
                                .filter_map(|obstacle| if *obstacle.position <
                                                          *car.position + 0.1 {
                                    dangerous = true;
                                    None
                                } else {
                                    Some(OrderedFloat(intelligent_acceleration(car, obstacle, 1.0)))
                                })
                                .min()
                                .unwrap();

                            let transfer_before_end_velocity =
                                (self.construction.length + 1.0 - *car.position) / 1.5;
                            let transfer_before_end_acceleration = transfer_before_end_velocity -
                                                                   car.velocity;

                            (next_obstacle_acceleration.min(transfer_before_end_acceleration),
                             dangerous)
                        };

                        let car = &mut self.microtraffic.cars[c];
                        car.acceleration = acceleration;

                        if dangerous && !car.cancelling {
                            car.transfer_acceleration = -car.transfer_acceleration;
                            car.cancelling = true;
                        }
                    }
                }

                for car in &mut self.microtraffic.cars {
                    *car.position += dt * car.velocity;
                    car.velocity =
                        (car.velocity + dt * car.acceleration).min(car.max_velocity).max(0.0);
                    car.transfer_position += dt * car.transfer_velocity;
                    car.transfer_velocity += dt * car.transfer_acceleration;
                    if car.transfer_velocity.abs() > car.velocity / 12.0 {
                        car.transfer_velocity = car.velocity / 12.0 *
                                                car.transfer_velocity.signum();
                    }
                }

                for obstacle in self.microtraffic
                    .left_obstacles
                    .iter_mut()
                    .chain(self.microtraffic.right_obstacles.iter_mut()) {
                    *obstacle.position += dt * obstacle.velocity;
                }

                if self.microtraffic.cars.len() > 1 {
                    for i in (0..self.microtraffic.cars.len() - 1).rev() {
                        if self.microtraffic.cars[i].position >
                           self.microtraffic.cars[i + 1].position {
                            self.microtraffic.cars.swap(i, i + 1);
                        }
                    }
                }

                if let (Some((left, left_start)), Some((right, right_start))) =
                    (self.connectivity.left, self.connectivity.right) {
                    let mut i = 0;
                    loop {
                        let (should_remove, done) =
                            if let Some(car) = self.microtraffic.cars.get(i) {
                                if car.transfer_position > 1.0 ||
                                   (*car.position > self.construction.length &&
                                    car.transfer_acceleration > 0.0) {
                                    right << AddCar{car: car.as_lane_car.offset_by(
                                right_start + self.self_to_interaction_offset(*car.position, false)
                            ), from: Some(self.id())};
                                    (true, false)
                                } else if car.transfer_position < -1.0 ||
                                          (*car.position > self.construction.length &&
                                           car.transfer_acceleration <= 0.0) {
                                    left << AddCar{car: car.as_lane_car.offset_by(
                                left_start + self.self_to_interaction_offset(*car.position, true)
                            ), from: Some(self.id())};
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

                    if (current_tick + 1) % TRAFFIC_LOGIC_THROTTLING ==
                       left.sub_actor_id as usize % TRAFFIC_LOGIC_THROTTLING {
                        let obstacles = self.microtraffic
                            .cars
                            .iter()
                            .filter_map(|car| if car.transfer_position < 0.3 ||
                                                 car.transfer_acceleration < 0.0 {
                                Some(car.as_obstacle.offset_by(
                                left_start + self.self_to_interaction_offset(*car.position, true)
                            ))
                            } else {
                                None
                            })
                            .collect();
                        left <<
                        AddObstacles {
                            obstacles: obstacles,
                            from: self.id(),
                        };
                    }

                    if (current_tick + 1) % TRAFFIC_LOGIC_THROTTLING ==
                       right.sub_actor_id as usize % TRAFFIC_LOGIC_THROTTLING {
                        let obstacles = self.microtraffic
                            .cars
                            .iter()
                            .filter_map(|car| if car.transfer_position > -0.3 ||
                                                 car.transfer_acceleration > 0.0 {
                                Some(car.as_obstacle.offset_by(
                                    right_start + self.self_to_interaction_offset(*car.position,
                                                                                  false)
                                ))
                            } else {
                                None
                            })
                            .collect();
                        right <<
                        AddObstacles {
                            obstacles: obstacles,
                            from: self.id(),
                        };
                    }
                }

                Fate::Live
            }
        }
    }
}

impl Recipient<SignalChanged> for Lane {
    fn receive(&mut self, msg: &SignalChanged) -> Fate {
        match *msg {
            SignalChanged { from, green } => {
                if let Some(interaction) =
                    self.connectivity
                        .interactions
                        .iter_mut()
                        .find(|interaction| match **interaction {
                            Interaction { partner_lane,
                                          kind: InteractionKind::Next { .. },
                                          .. } => partner_lane == from,
                            _ => false,
                        }) {
                    interaction.kind = InteractionKind::Next { green: green }
                } else {
                    println!("Lane doesn't know about next lane yet");
                }
                Fate::Live
            }
        }
    }
}

pub fn setup() {
    Swarm::<Lane>::handle::<AddCar>();
    Swarm::<Lane>::handle::<AddObstacles>();
    Swarm::<Lane>::handle::<Tick>();
    Swarm::<Lane>::handle::<SignalChanged>();

    Swarm::<TransferLane>::handle::<AddCar>();
    Swarm::<TransferLane>::handle::<AddObstacles>();
    Swarm::<TransferLane>::handle::<Tick>();
}
