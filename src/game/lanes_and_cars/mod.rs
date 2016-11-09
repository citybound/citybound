pub mod ui;
pub mod planning;
mod intelligent_acceleration;
use self::intelligent_acceleration::{intelligent_acceleration, COMFORTABLE_BREAKING_DECELERATION};
use core::geometry::CPath;
use kay::{ID, CVec, Swarm, Recipient, ActorSystem, Fate};
use descartes::{FiniteCurve};
use ordered_float::OrderedFloat;
use itertools::Itertools;
use ::std::f32::INFINITY;
use ::std::ops::{Deref, DerefMut};

#[derive(Compact, Actor, Clone)]
pub struct Lane {
    _id: ID,
    length: f32,
    path: CPath,
    interactions: CVec<Interaction>,
    interaction_obstacles: CVec<Obstacle>,
    cars: CVec<LaneCar>
}

impl Lane {
    fn new(path: CPath) -> Self {
        Lane {
            _id: ID::invalid(),
            length: path.length(),
            path: path,
            interactions: CVec::new(),
            interaction_obstacles: CVec::new(),
            cars: CVec::new()
        }
    }

    fn add_next_lane(&mut self, next_lane: ID) {
        self.interactions.push(Interaction{
            partner_lane: next_lane,
            kind: Next{partner_start: 0.0}
        });
    }

    fn add_previous_lane(&mut self, previous_lane: ID, previous_lane_length: f32) {
        self.interactions.push(Interaction{
            partner_lane: previous_lane,
            kind: Previous{start: 0.0, partner_length: previous_lane_length}
        });
    } 
}

#[derive(Compact, Actor, Clone)]
pub struct TransferLane {
    _id: ID,
    length: f32,
    path: CPath,
    left: ID,
    left_start: f32,
    right: ID,
    right_start: f32,
    interaction_obstacles: CVec<Obstacle>,
    cars: CVec<TransferringLaneCar>
}

impl TransferLane {
    fn new(path: CPath, left: ID, left_start: f32, right: ID, right_start: f32) -> TransferLane {
        TransferLane{
            _id: ID::invalid(),
            length: path.length(),
            path: path,
            left: left,
            left_start: left_start,
            right: right,
            right_start: right_start,
            interaction_obstacles: CVec::new(),
            cars: CVec::new()
        }
    }
}

#[derive(Copy, Clone)]
enum Add{
    Car(LaneCar),
    InteractionObstacle(Obstacle)
}

impl Recipient<Add> for Lane {
    fn receive(&mut self, msg: &Add) -> Fate {match *msg{
        Add::Car(car) => {
            // TODO: optimize using BinaryHeap?
            self.cars.push(car);
            self.cars.sort_by_key(|car| car.as_obstacle.position);
            Fate::Live
        },
        Add::InteractionObstacle(obstacle) => {
            self.interaction_obstacles.push(obstacle);
            Fate::Live
        }
    }}
}

impl Recipient<Add> for TransferLane {
    fn receive(&mut self, msg: &Add) -> Fate {match *msg{
        Add::Car(car) => {
            self.cars.push(TransferringLaneCar{
                as_lane_car: car,
                transfer_position: -1.0,
                transfer_velocity: 0.0,
                transfer_acceleration: 0.1
            });
            // TODO: optimize using BinaryHeap?
            self.cars.sort_by_key(|car| car.as_obstacle.position);
            Fate::Live
        },
        Add::InteractionObstacle(obstacle) => {
            self.interaction_obstacles.push(obstacle);
            Fate::Live
        },
    }}
}

use core::simulation::Tick;

impl Recipient<Tick> for Lane {
    fn receive(&mut self, msg: &Tick) -> Fate {match *msg{
        Tick{dt} => {
            // TODO: optimize using BinaryHeap?
            self.interaction_obstacles.sort_by_key(|obstacle| obstacle.position);

            for c in 0..self.cars.len() {
                let next_obstacle = self.cars.get(c + 1).map_or(Obstacle::far_ahead(), |car| car.as_obstacle);
                let car = &mut self.cars[c];
                let next_obstacle_acceleration = intelligent_acceleration(car, &next_obstacle);

                // TODO: optimize, avoid nested loop
                let next_overlap_obstacle_acceleration = self.interaction_obstacles.iter()
                    .find(|obstacle| obstacle.position > car.position)
                    .map(|obstacle| intelligent_acceleration(car, obstacle));

                car.acceleration = next_obstacle_acceleration.min(next_overlap_obstacle_acceleration.unwrap_or(INFINITY));
            }

            for car in &mut self.cars {
                *car.position += dt * car.velocity;
                car.velocity = (car.velocity + dt * car.acceleration).min(car.max_velocity).max(0.0);
            }
            
            loop {
                let should_pop = self.cars.iter().rev().find(|car| *car.position > self.length).map(|car_over_end| {
                    if let Some(next_overlap) = self.interactions.iter().find(|overlap| match overlap.kind {Next{..} => true, _ => false}) {
                        next_overlap.partner_lane << Add::Car(car_over_end.offset_by(-self.length));
                    };
                    car_over_end
                }).is_some();
                if should_pop {self.cars.pop();} else {break;}
            }

            for interaction in self.interactions.iter() {
                let mut cars = self.cars.iter();
                let send_obstacle = |obstacle: Obstacle| interaction.partner_lane << Add::InteractionObstacle(obstacle);
                
                match interaction.kind {
                    Overlap{start, end, partner_start, kind, ..} => {
                        match kind {
                            Parallel => cars.filter(|car: &&LaneCar| *car.position > start && *car.position < end).map(|car|
                                car.as_obstacle.offset_by(-start + partner_start)
                            ).foreach(send_obstacle),
                            Conflicting => if cars.any(|car: &LaneCar| *car.position > start && *car.position < end) {
                                (send_obstacle)(Obstacle{position: OrderedFloat(partner_start), velocity: 0.0, max_velocity: 0.0})
                            }
                        }
                    }
                    Previous{start, partner_length} =>
                        if let Some(next_car) = cars.find(|car| *car.position > start) {
                            (send_obstacle)(next_car.as_obstacle.offset_by(-start + partner_length))
                        },
                    Next{..} => {
                        //TODO: for looking backwards for merging lanes?
                    }
                };
            }

            self.interaction_obstacles.clear();
            Fate::Live
        }
    }}
}

impl Recipient<Tick> for TransferLane {
    fn receive(&mut self, msg: &Tick) -> Fate {match *msg{
        Tick{dt} => {
            self.interaction_obstacles.sort_by_key(|obstacle| obstacle.position);

            for c in 0..self.cars.len() {
                let (acceleration, is_dangerous) = {
                    let car = &self.cars[c];
                    
                    let next_obstacle = self.cars.get(c + 1).map_or(Obstacle::far_ahead(), |car| car.as_obstacle);
                    let previous_obstacle = if c > 0 {self.cars[c - 1].as_obstacle} else {Obstacle::far_behind()};

                    let next_interaction_obstacle_index = self.interaction_obstacles.iter().position(
                        |obstacle| obstacle.position > car.position
                    );
                    let next_interaction_obstacle = next_interaction_obstacle_index
                        .map(|idx| self.interaction_obstacles[idx]).unwrap_or_else(Obstacle::far_ahead);
                    let previous_interaction_obstacle = next_interaction_obstacle_index
                        .and_then(|idx| self.interaction_obstacles.get(idx - 1)).cloned().unwrap_or_else(Obstacle::far_behind);

                    let next_obstacle_acceleration = intelligent_acceleration(car, &next_obstacle)
                        .min(intelligent_acceleration(car, &next_interaction_obstacle));
                    let previous_obstacle_acceleration = intelligent_acceleration(&previous_obstacle, &car.as_obstacle)
                        .min(intelligent_acceleration(&previous_interaction_obstacle, &car.as_obstacle));

                    let politeness_factor = 0.3;

                    let acceleration = if previous_obstacle_acceleration < 0.0 {
                        (1.0 - politeness_factor) * next_obstacle_acceleration + politeness_factor * previous_obstacle_acceleration
                    } else {
                        next_obstacle_acceleration
                    };

                    let is_dangerous = next_obstacle_acceleration < -2.0 * COMFORTABLE_BREAKING_DECELERATION
                        || previous_obstacle_acceleration < -2.0 * COMFORTABLE_BREAKING_DECELERATION;

                    (acceleration, is_dangerous)
                };

                let car = &mut self.cars[c];
                car.acceleration = acceleration;
                if is_dangerous {
                    car.transfer_acceleration = if car.transfer_position >= 0.0 {0.3} else {-0.3}
                }
                // smooth out arrival on other lane
                #[allow(float_cmp)]
                let arriving_soon = car.transfer_velocity.abs() > 0.1 && car.transfer_position.abs() > 0.5 && car.transfer_position.signum() == car.transfer_velocity.signum();
                if arriving_soon {
                    car.transfer_acceleration = -0.9 * car.transfer_velocity;
                }
            }

            for car in &mut self.cars {
                *car.position += dt * car.velocity;
                car.velocity = (car.velocity + dt * car.acceleration).min(car.max_velocity).max(0.0);
                car.transfer_position += dt * car.transfer_velocity;
                car.transfer_velocity += dt * car.transfer_acceleration;
                if car.transfer_velocity.abs() > car.velocity/12.0 {
                    car.transfer_velocity = car.velocity/12.0 * car.transfer_velocity.signum();
                }
            }

            let mut i = 0;
            loop {
                let (should_remove, done) = if let Some(car) = self.cars.get(i) {
                    if car.transfer_position < -1.0 {
                        self.left << Add::Car(car.as_lane_car.offset_by(self.left_start));
                        (true, false)
                    } else if car.transfer_position > 1.0 {
                        self.right << Add::Car(car.as_lane_car.offset_by(self.right_start));
                        (true, false)
                    } else {
                        i += 1;
                        (false, false)
                    }
                } else {
                    (false, true)
                };
                if done {break;}
                if should_remove {self.cars.remove(i);}
            }

            for car in &self.cars {
                if car.transfer_position < 0.3 || car.transfer_velocity < 0.0 {
                    self.left << Add::InteractionObstacle(car.as_obstacle.offset_by(self.left_start));
                }
                if car.transfer_position > -0.3 || car.transfer_velocity > 0.0 {
                    self.right << Add::InteractionObstacle(car.as_obstacle.offset_by(self.right_start));
                }
            }

            self.interaction_obstacles.clear();
            Fate::Live
        }
    }}
}

pub fn setup(system: &mut ActorSystem) {
    system.add_individual(Swarm::<Lane>::new());
    system.add_inbox::<Add, Swarm<Lane>>();
    system.add_inbox::<Tick, Swarm<Lane>>();

    system.add_individual(Swarm::<TransferLane>::new());
    system.add_inbox::<Add, Swarm<TransferLane>>();
    system.add_inbox::<Tick, Swarm<TransferLane>>();
}

#[derive(Copy, Clone)]
pub struct Obstacle {
    position: OrderedFloat<f32>,
    velocity: f32,
    max_velocity: f32
}

impl Obstacle {
    fn far_ahead() -> Obstacle {Obstacle{position: OrderedFloat(INFINITY), velocity: INFINITY, max_velocity: INFINITY}}
    fn far_behind() -> Obstacle {Obstacle{position: OrderedFloat(-INFINITY), velocity: 0.0, max_velocity: 20.0}}
    fn offset_by(&self, delta: f32) -> Obstacle {
        Obstacle{
            position: OrderedFloat(*self.position + delta),
            .. *self
        }
    } 
}

#[derive(Copy, Clone)]
pub struct LaneCar {
    trip: ID,
    as_obstacle: Obstacle,
    acceleration: f32
}

impl LaneCar {
    fn offset_by(&self, delta: f32) -> LaneCar {
        LaneCar{
            as_obstacle: self.as_obstacle.offset_by(delta),
            .. *self
        }
    }
}

impl Deref for LaneCar {
    type Target = Obstacle;

    fn deref(&self) -> &Obstacle {&self.as_obstacle}
}

impl DerefMut for LaneCar {
    fn deref_mut(&mut self) -> &mut Obstacle {&mut self.as_obstacle}
}

#[derive(Copy, Clone)]
struct TransferringLaneCar {
    as_lane_car: LaneCar,
    transfer_position: f32,
    transfer_velocity: f32,
    transfer_acceleration: f32
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
struct Interaction {
    partner_lane: ID,
    kind: InteractionKind
}

#[derive(Copy, Clone)]
enum InteractionKind{
    Overlap{
        start: f32,
        end: f32,
        partner_start: f32,
        partner_end: f32,
        kind: OverlapKind
    },
    Next{
        partner_start: f32
    },
    Previous{
        start: f32,
        partner_length: f32
    }
}
use self::InteractionKind::{Overlap, Next, Previous};

#[derive(Copy, Clone)]
enum OverlapKind{Parallel, Conflicting}
use self::OverlapKind::{Parallel, Conflicting};