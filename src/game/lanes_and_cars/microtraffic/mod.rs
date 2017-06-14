use kay::{ID, ActorSystem, Fate};
use kay::swarm::{Swarm, SubActor};
use compact::CVec;
use ordered_float::OrderedFloat;
use std::f32::INFINITY;
use std::ops::{Deref, DerefMut};

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
        Obstacle {
            position: OrderedFloat(*self.position + delta),
            ..*self
        }
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

pub fn setup(system: &mut ActorSystem) {
    system.extend(Swarm::<Lane>::subactors(|mut each_lane| {
        each_lane.on(|&AddCar { car, .. }, lane, _| {
            // TODO: horrible hack to encode it like this
            let car_forcibly_spawned = *car.as_obstacle.position < 0.0;

            let maybe_next_hop_interaction =
                lane.pathfinding
                    .routes
                    .get(car.destination)
                    .or_else(|| {
                        lane.pathfinding
                            .routes
                            .get(car.destination.landmark_destination())
                    })
                    .or_else(|| {
                        println!("NO ROUTE!");
                        if car_forcibly_spawned || lane.pathfinding.routes.is_empty() {
                            None
                        } else {
                            // pseudorandom, lol
                            lane.pathfinding
                                .routes
                                .values()
                                .nth((car.velocity * 10000.0) as usize %
                                     lane.pathfinding.routes.len())
                        }
                    })
                    .map(|&RoutingInfo { outgoing_idx, .. }| outgoing_idx as usize);

            let spawn_possible = if car_forcibly_spawned {
                if lane.last_spawn_position > 2.0 {
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
                        lane.last_spawn_position -= 6.0;
                        car.as_obstacle
                            .offset_by(-*car.as_obstacle.position)
                            .offset_by(lane.last_spawn_position + 6.0)
                    } else {
                        car.as_obstacle
                    },
                    ..car
                };

                // TODO: optimize using BinaryHeap?
                let maybe_next_car_position =
                    lane.microtraffic.cars.iter().position(|other_car| {
                        other_car.as_obstacle.position > car.as_obstacle.position
                    });
                match maybe_next_car_position {
                    Some(next_car_position) => {
                        lane.microtraffic.cars.insert(next_car_position, routed_car)
                    }
                    None => lane.microtraffic.cars.push(routed_car),
                }
            } else {
                // TODO: cancel trip
            }

            Fate::Live

        });

        each_lane.on(|&AddObstacles { ref obstacles, from }, lane, _| {
            lane.microtraffic
                .obstacles
                .retain(|&(_, received_from)| received_from != from);
            lane.microtraffic
                .obstacles
                .extend(obstacles.iter().map(|obstacle| (*obstacle, from)));
            Fate::Live
        });

        each_lane.on(|&Tick { dt, current_tick }, lane, world| {
            lane.construction.progress += dt * 400.0;

            let do_traffic = current_tick.ticks() % TRAFFIC_LOGIC_THROTTLING ==
                             lane.id().sub_actor_id as usize % TRAFFIC_LOGIC_THROTTLING;

            let old_green = lane.microtraffic.green;
            lane.microtraffic.yellow_to_red = if lane.microtraffic.timings.is_empty() {
                true
            } else {
                !lane.microtraffic.timings[((current_tick.ticks() + 100) / 25) %
                                           lane.microtraffic.timings.len()]
            };
            lane.microtraffic.yellow_to_green = if lane.microtraffic.timings.is_empty() {
                true
            } else {
                lane.microtraffic.timings[((current_tick.ticks() + 100) / 25) %
                                          lane.microtraffic.timings.len()]
            };
            lane.microtraffic.green = if lane.microtraffic.timings.is_empty() {
                true
            } else {
                lane.microtraffic.timings[(current_tick.ticks() / 25) %
                                          lane.microtraffic.timings.len()]
            };

            // TODO: this is just a hacky way to update new lanes about existing lane's green
            if old_green != lane.microtraffic.green || do_traffic {
                for interaction in &lane.connectivity.interactions {
                    if let Interaction {
                               kind: InteractionKind::Previous { .. },
                               partner_lane,
                               ..
                           } = *interaction {
                        world.send(partner_lane,
                                   SignalChanged {
                                       from: lane.id(),
                                       green: lane.microtraffic.green,
                                   });
                    }
                }
            }

            if current_tick.ticks() % PATHFINDING_THROTTLING ==
               lane.id().sub_actor_id as usize % PATHFINDING_THROTTLING {
                self::pathfinding::tick(lane, world);
            }

            if do_traffic {
                // TODO: optimize using BinaryHeap?
                lane.microtraffic
                    .obstacles
                    .sort_by_key(|&(ref obstacle, _id)| obstacle.position);

                let mut obstacles = lane.microtraffic
                    .obstacles
                    .iter()
                    .map(|&(ref obstacle, _id)| obstacle);
                let mut maybe_next_obstacle = obstacles.next();

                for c in 0..lane.microtraffic.cars.len() {
                    let next_obstacle = lane.microtraffic
                        .cars
                        .get(c + 1)
                        .map_or(Obstacle::far_ahead(), |car| car.as_obstacle);
                    let car = &mut lane.microtraffic.cars[c];
                    let next_car_acceleration = intelligent_acceleration(car, &next_obstacle, 2.0);

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

                    if let Interaction {
                               start,
                               kind: InteractionKind::Next { green },
                               ..
                           } = lane.connectivity.interactions[car.next_hop_interaction as usize] {
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

            for car in &mut lane.microtraffic.cars {
                *car.position += dt * car.velocity;
                car.velocity = (car.velocity + dt * car.acceleration)
                    .min(car.max_velocity)
                    .max(0.0);
            }

            for &mut (ref mut obstacle, _id) in &mut lane.microtraffic.obstacles {
                *obstacle.position += dt * obstacle.velocity;
            }

            if lane.microtraffic.cars.len() > 1 {
                for i in (0..lane.microtraffic.cars.len() - 1).rev() {
                    lane.microtraffic.cars[i].position =
                        OrderedFloat((*lane.microtraffic.cars[i].position)
                                         .min(*lane.microtraffic.cars[i + 1].position));
                }
            }

            loop {
                let maybe_switch_car = lane.microtraffic
                    .cars
                    .iter()
                    .enumerate()
                    .rev()
                    .filter_map(|(i, &car)| {
                        let interaction = lane.connectivity.interactions[car.next_hop_interaction as
                                                                         usize];

                        match interaction.kind {
                            InteractionKind::Overlap {
                                end, kind: OverlapKind::Transfer, ..
                            } => {
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

                if let Some((idx_to_remove, next_lane, start, partner_start)) = maybe_switch_car {
                    let car = lane.microtraffic.cars.remove(idx_to_remove);
                    if lane.id() != car.destination.node {
                        world.send(next_lane,
                                   AddCar {
                                       car: car.offset_by(partner_start - start),
                                       from: Some(lane.id()),
                                   });
                    }
                } else {
                    break;
                }
            }

            // ASSUMPTION: only one interaction per Lane/Lane pair
            for interaction in lane.connectivity.interactions.iter() {
                let cars = lane.microtraffic.cars.iter();

                if (current_tick.ticks() + 1) % TRAFFIC_LOGIC_THROTTLING ==
                   interaction.partner_lane.sub_actor_id as usize % TRAFFIC_LOGIC_THROTTLING {
                    let maybe_obstacles =
                        obstacles_for_interaction(interaction,
                                                  cars,
                                                  lane.microtraffic.obstacles.iter());

                    if let Some(obstacles) = maybe_obstacles {
                        world.send(interaction.partner_lane,
                                   AddObstacles { obstacles: obstacles, from: lane.id() })
                    }
                }
            }

            Fate::Live
        });

        each_lane.on(|&SignalChanged { from, green }, lane, _| {
            if let Some(interaction) =
                lane.connectivity
                    .interactions
                    .iter_mut()
                    .find(|interaction| match **interaction {
                              Interaction {
                                  partner_lane,
                                  kind: InteractionKind::Next { .. },
                                  ..
                              } => partner_lane == from,
                              _ => false,
                          }) {
                interaction.kind = InteractionKind::Next { green: green }
            } else {
                println!("Lane doesn't know about next lane yet");
            }
            Fate::Live
        })
    }));

    system.extend(Swarm::<TransferLane>::subactors(|mut each_t_lane| {
        each_t_lane.on(|&AddCar { car, from: maybe_from }, lane, _| {
            let from = maybe_from.expect("car has to come from somewhere on transfer lane");

            let from_left = from == lane.connectivity.left.expect("should have a left lane").0;
            let side_multiplier = if from_left { -1.0 } else { 1.0 };
            let offset = lane.interaction_to_self_offset(*car.position, from_left);
            lane.microtraffic.cars.push(TransferringLaneCar {
                                            as_lane_car: car.offset_by(offset),
                                            transfer_position: 1.0 * side_multiplier,
                                            transfer_velocity: 0.0,
                                            transfer_acceleration: 0.3 * -side_multiplier,
                                            cancelling: false,
                                        });
            // TODO: optimize using BinaryHeap?
            lane.microtraffic
                .cars
                .sort_by_key(|car| car.as_obstacle.position);
            Fate::Live
        });

        each_t_lane.on(|&AddObstacles { ref obstacles, from }, lane, _| {
            if let (Some((left_id, _)), Some(_)) =
                (lane.connectivity.left, lane.connectivity.right) {
                if left_id == from {
                    lane.microtraffic.left_obstacles = obstacles
                        .iter()
                        .map(|obstacle| {
                            obstacle.offset_by(lane.interaction_to_self_offset(*obstacle.position,
                                                                               true))
                        })
                        .collect();
                } else {
                    lane.microtraffic.right_obstacles = obstacles
                        .iter()
                        .map(|obstacle| {
                            obstacle.offset_by(lane.interaction_to_self_offset(*obstacle.position,
                                                                               false))
                        })
                        .collect();
                };
            } else {
                println!("transfer lane not connected for obstacles yet");
            }
            Fate::Live
        });

        each_t_lane.on(|&Tick { dt, current_tick }, lane, world| {
            lane.construction.progress += dt * 400.0;

            let do_traffic = current_tick.ticks() % TRAFFIC_LOGIC_THROTTLING ==
                             lane.id().sub_actor_id as usize % TRAFFIC_LOGIC_THROTTLING;

            if do_traffic {
                // TODO: optimize using BinaryHeap?
                lane.microtraffic
                    .left_obstacles
                    .sort_by_key(|obstacle| obstacle.position);
                lane.microtraffic
                    .right_obstacles
                    .sort_by_key(|obstacle| obstacle.position);

                for c in 0..lane.microtraffic.cars.len() {
                    let (acceleration, dangerous) = {
                        let car = &lane.microtraffic.cars[c];
                        let next_car = lane.microtraffic
                            .cars
                            .iter()
                            .find(|other_car| *other_car.position > *car.position)
                            .map(|other_car| &other_car.as_obstacle);

                        let maybe_next_left_obstacle = if car.transfer_position < 0.3 ||
                                                          car.transfer_acceleration < 0.0 {
                            lane.microtraffic
                                .left_obstacles
                                .iter()
                                .find(|obstacle| *obstacle.position + 5.0 > *car.position)
                        } else {
                            None
                        };

                        let maybe_next_right_obstacle = if car.transfer_position > -0.3 ||
                                                           car.transfer_acceleration > 0.0 {
                            lane.microtraffic
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
                            .filter_map(|obstacle| if *obstacle.position < *car.position + 0.1 {
                                            dangerous = true;
                                            None
                                        } else {
                                            Some(OrderedFloat(intelligent_acceleration(car,
                                                                                       obstacle,
                                                                                       1.0)))
                                        })
                            .min()
                            .unwrap();

                        let transfer_before_end_velocity =
                            (lane.construction.length + 1.0 - *car.position) / 1.5;
                        let transfer_before_end_acceleration = transfer_before_end_velocity -
                                                               car.velocity;

                        (next_obstacle_acceleration.min(transfer_before_end_acceleration),
                         dangerous)
                    };

                    let car = &mut lane.microtraffic.cars[c];
                    car.acceleration = acceleration;

                    if dangerous && !car.cancelling {
                        car.transfer_acceleration = -car.transfer_acceleration;
                        car.cancelling = true;
                    }
                }
            }

            for car in &mut lane.microtraffic.cars {
                *car.position += dt * car.velocity;
                car.velocity = (car.velocity + dt * car.acceleration)
                    .min(car.max_velocity)
                    .max(0.0);
                car.transfer_position += dt * car.transfer_velocity;
                car.transfer_velocity += dt * car.transfer_acceleration;
                if car.transfer_velocity.abs() > car.velocity / 12.0 {
                    car.transfer_velocity = car.velocity / 12.0 * car.transfer_velocity.signum();
                }
            }

            for obstacle in lane.microtraffic
                    .left_obstacles
                    .iter_mut()
                    .chain(lane.microtraffic.right_obstacles.iter_mut()) {
                *obstacle.position += dt * obstacle.velocity;
            }

            if lane.microtraffic.cars.len() > 1 {
                for i in (0..lane.microtraffic.cars.len() - 1).rev() {
                    if lane.microtraffic.cars[i].position > lane.microtraffic.cars[i + 1].position {
                        lane.microtraffic.cars.swap(i, i + 1);
                    }
                }
            }

            if let (Some((left, left_start)), Some((right, right_start))) =
                (lane.connectivity.left, lane.connectivity.right) {
                let mut i = 0;
                loop {
                    let (should_remove, done) = if let Some(car) = lane.microtraffic.cars.get(i) {
                        if car.transfer_position > 1.0 ||
                           (*car.position > lane.construction.length &&
                            car.transfer_acceleration > 0.0) {
                            world.send(right,
                            AddCar {
                                car: car.as_lane_car
                                    .offset_by(right_start +
                                               lane.self_to_interaction_offset(*car.position,
                                                                               false)),
                                from: Some(lane.id()),
                            });
                            (true, false)
                        } else if car.transfer_position < -1.0 ||
                                  (*car.position > lane.construction.length &&
                                   car.transfer_acceleration <= 0.0) {
                            world.send(left,
                            AddCar {
                                car: car.as_lane_car
                                    .offset_by(left_start +
                                               lane.self_to_interaction_offset(*car.position,
                                                                               true)),
                                from: Some(lane.id()),
                            });
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
                        lane.microtraffic.cars.remove(i);
                    }
                }

                if (current_tick.ticks() + 1) % TRAFFIC_LOGIC_THROTTLING ==
                   left.sub_actor_id as usize % TRAFFIC_LOGIC_THROTTLING {
                    let obstacles = lane.microtraffic
                        .cars
                        .iter()
                        .filter_map(|car| if car.transfer_position < 0.3 ||
                                 car.transfer_acceleration < 0.0 {
                            Some(car.as_obstacle
                                     .offset_by(left_start +
                                                lane.self_to_interaction_offset(*car.position,
                                                                                true)))
                        } else {
                            None
                        })
                        .collect();
                    world.send(left, AddObstacles { obstacles: obstacles, from: lane.id() });
                }

                if (current_tick.ticks() + 1) % TRAFFIC_LOGIC_THROTTLING ==
                   right.sub_actor_id as usize % TRAFFIC_LOGIC_THROTTLING {
                    let obstacles = lane.microtraffic
                        .cars
                        .iter()
                        .filter_map(|car| if car.transfer_position > -0.3 ||
                                 car.transfer_acceleration > 0.0 {
                            Some(car.as_obstacle
                                     .offset_by(right_start +
                                                lane.self_to_interaction_offset(*car.position,
                                                                                false)))
                        } else {
                            None
                        })
                        .collect();
                    world.send(right,
                               AddObstacles { obstacles: obstacles, from: lane.id() });
                }
            }

            Fate::Live
        })
    }))
}

use core::simulation::Tick;

const TRAFFIC_LOGIC_THROTTLING: usize = 30;
const PATHFINDING_THROTTLING: usize = 10;

#[derive(Copy, Clone)]
pub struct SignalChanged {
    from: ID,
    green: bool,
}

fn obstacles_for_interaction(interaction: &Interaction,
                             mut cars: ::std::slice::Iter<LaneCar>,
                             self_obstacles_iter: ::std::slice::Iter<(Obstacle, ID)>)
                             -> Option<CVec<Obstacle>> {
    match *interaction {
        Interaction {
            partner_lane,
            start,
            partner_start,
            kind: InteractionKind::Overlap { end, kind, .. },
            ..
        } => {
            Some(match kind {
                OverlapKind::Parallel => {
                    cars.skip_while(|car: &&LaneCar| *car.position + 2.0 * car.velocity < start)
                        .take_while(|car: &&LaneCar| *car.position < end)
                        .map(|car| car.as_obstacle.offset_by(-start + partner_start))
                        .collect()
                }
                OverlapKind::Transfer => {
                    cars.skip_while(|car: &&LaneCar| *car.position + 2.0 * car.velocity < start)
                        .map(|car| car.as_obstacle.offset_by(-start + partner_start))
                        .chain(self_obstacles_iter.filter_map(
                            |&(obstacle, id)| if id != partner_lane &&
                                                 *obstacle.position + 2.0 * obstacle.velocity >
                                                 start {
                                Some(obstacle.offset_by(-start + partner_start))
                            } else {
                                None
                            },
                        ))
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
                             }].into()
                    } else {
                        CVec::new()
                    }
                }
            })
        }
        Interaction {
            start,
            partner_start,
            kind: InteractionKind::Previous,
            ..
        } => {
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
