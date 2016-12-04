pub mod lane_rendering;
pub mod lane_thing_collector;
pub mod planning;
pub mod pathfinding;
mod intelligent_acceleration;
use self::intelligent_acceleration::{intelligent_acceleration, COMFORTABLE_BREAKING_DECELERATION};
use core::geometry::{CPath, add_debug_path};
use kay::{ID, Actor, CVec, Swarm, CreateWith, Recipient, ActorSystem, Fate};
use descartes::{FiniteCurve, RoughlyComparable, Band, Intersect, Curve, Path, Dot, WithUniqueOrthogonal};
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
    cars: CVec<LaneCar>,
    in_construction: f32,
    on_intersection: bool,
    pathfinding_info: pathfinding::PathfindingInfo
}

impl Lane {
    pub fn new(path: CPath, on_intersection: bool) -> Self {
        Lane {
            _id: ID::invalid(),
            length: path.length(),
            path: path,
            interactions: CVec::new(),
            interaction_obstacles: CVec::new(),
            cars: CVec::new(),
            in_construction: 0.0,
            on_intersection: on_intersection,
            pathfinding_info: pathfinding::PathfindingInfo::default()
        }
    }
}

#[derive(Compact, Actor, Clone)]
pub struct TransferLane {
    _id: ID,
    length: f32,
    path: CPath,
    left: Option<(ID, f32)>,
    right: Option<(ID, f32)>,
    interaction_obstacles: CVec<Obstacle>,
    cars: CVec<TransferringLaneCar>,
    in_construction: f32
}

impl TransferLane {
    fn new(path: CPath) -> TransferLane {
        TransferLane{
            _id: ID::invalid(),
            length: path.length(),
            path: path,
            left: None,
            right: None,
            interaction_obstacles: CVec::new(),
            cars: CVec::new(),
            in_construction: 0.0
        }
    }
}

#[derive(Copy, Clone)]
enum Add{
    Car(LaneCar, Option<ID>),
    InteractionObstacle(Obstacle)
}

use self::pathfinding::RoutingInfo;

impl Recipient<Add> for Lane {
    fn receive(&mut self, msg: &Add) -> Fate {match *msg{
        Add::Car(car, _from) => {
            let next_hop_interaction = self.pathfinding_info.routes.get(car.destination).map(|&RoutingInfo{outgoing_idx, ..}|
                Some(outgoing_idx as usize)
            ).unwrap_or_else(|| {
                println!("NO ROUTE! Routes: {:#?}", self.pathfinding_info.routes.pairs().collect::<Vec<_>>());
                self.interactions.iter().position(|interaction| match interaction.kind {Next{..} => true, _ => false})
            }).expect("the car should be able to go somewhere!");

            println!("next hop idx will be {:?}", next_hop_interaction);

            let routed_car = LaneCar{
                next_hop_interaction: next_hop_interaction as u8,
                .. car
            };

            // TODO: optimize using BinaryHeap?
            let maybe_next_car_position = self.cars.iter().position(|other_car| other_car.as_obstacle.position > car.as_obstacle.position);
            match maybe_next_car_position {
                Some(next_car_position) => self.cars.insert(next_car_position, routed_car),
                None => self.cars.push(routed_car)
            }
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
        Add::Car(car, Some(from)) => {
            let side_multiplier = if from == self.left.expect("should have a left lane").0 {1.0} else {-1.0};
            self.cars.push(TransferringLaneCar{
                as_lane_car: car,
                transfer_position: 1.0 * side_multiplier,
                transfer_velocity: 0.0,
                transfer_acceleration: 0.3 * -side_multiplier
            });
            // TODO: optimize using BinaryHeap?
            self.cars.sort_by_key(|car| car.as_obstacle.position);
            Fate::Live
        },
        Add::Car(_, None) => panic!("car has to come from somewhere on a transfer lane"),
        Add::InteractionObstacle(obstacle) => {
            self.interaction_obstacles.push(obstacle);
            Fate::Live
        },
    }}
}

use core::simulation::Tick;

const TRAFFIC_LOGIC_THROTTLING : usize = 30;
const PATHFINDING_THROTTLING : usize = 3;

impl Recipient<Tick> for Lane {
    fn receive(&mut self, msg: &Tick) -> Fate {match *msg{
        Tick{dt, current_tick} => {
            self.in_construction += dt * 400.0;

            if current_tick % PATHFINDING_THROTTLING == self.id().instance_id as usize % PATHFINDING_THROTTLING {
                self::pathfinding::tick(self);
            }

            let do_traffic = current_tick % TRAFFIC_LOGIC_THROTTLING == self.id().instance_id as usize % TRAFFIC_LOGIC_THROTTLING;

            if do_traffic {
                // TODO: optimize using BinaryHeap?
                self.interaction_obstacles.sort_by_key(|obstacle| obstacle.position);

                let mut overlap_obstacles = self.interaction_obstacles.iter();
                let mut maybe_next_overlap_obstacle = overlap_obstacles.next();

                for c in 0..self.cars.len() {
                    let next_obstacle = self.cars.get(c + 1).map_or(Obstacle::far_ahead(), |car| car.as_obstacle);
                    let car = &mut self.cars[c];
                    let next_obstacle_acceleration = intelligent_acceleration(car, &next_obstacle);
                    
                    maybe_next_overlap_obstacle = maybe_next_overlap_obstacle.and_then(|obstacle| {
                        let mut following_obstacle = Some(obstacle);
                        while following_obstacle.is_some() && following_obstacle.unwrap().position < car.position {
                            following_obstacle = overlap_obstacles.next();
                        }
                        following_obstacle
                    });
                    
                    let next_overlap_obstacle_acceleration = if let Some(next_overlap_obstacle) = maybe_next_overlap_obstacle {
                        intelligent_acceleration(car, next_overlap_obstacle)
                    } else {INFINITY};

                    car.acceleration = next_obstacle_acceleration.min(next_overlap_obstacle_acceleration);
                }
            }

            for car in &mut self.cars {
                *car.position += dt * car.velocity;
                car.velocity = (car.velocity + dt * car.acceleration).min(car.max_velocity).max(0.0);
            }

            if self.cars.len() > 1 {
                for i in (0..self.cars.len() - 1).rev() {
                    self.cars[i].position = OrderedFloat((*self.cars[i].position).min(*self.cars[i + 1].position));
                }
            }

            loop {
                let maybe_switch_car = self.cars.iter().enumerate().rev().filter_map(|(i, &car)| {
                    let interaction = self.interactions[car.next_hop_interaction as usize];
                    
                    if *car.position > interaction.start {
                        println!("interaction for switching: {:?} (idx {:?})", interaction, car.next_hop_interaction);
                        Some((i, interaction.partner_lane, interaction.start, interaction.partner_start))
                    } else {None}
                }).next();

                if let Some((idx_to_remove, next_lane, start, partner_start)) = maybe_switch_car {
                    let car = self.cars.remove(idx_to_remove);
                    println!("{:?} -> {:?}", self.id(), car.destination.node);
                    if self.id() == car.destination.node {
                        add_debug_path(self.path.clone(), [0.0, 1.0, 0.0], 0.4);
                    } else {
                        next_lane << Add::Car(car.offset_by(partner_start - start), Some(self.id()));
                        println!("switched car from {:?} to {:?}", self.id(), next_lane);
                        add_debug_path(self.path.clone(), [0.0, 0.0, 1.0], 0.4);
                    }
                } else {
                    break;
                }
            }
            
            // loop {
            //     let should_pop = self.cars.iter().rev().find(|car| *car.position > self.length).map(|car_over_end| {
            //         let first_next_interaction = self.interactions.iter().find(|interaction| match interaction.kind {Next{..} => true, _ => false});
            //         if let Some(&Interaction{partner_lane, kind: Next{partner_start}, ..}) = first_next_interaction {
            //             partner_lane << Add::Car(car_over_end.offset_by(-self.length + partner_start), Some(self.id()));
            //         };
            //         car_over_end
            //     }).is_some();
            //     if should_pop {self.cars.pop();} else {break;}
            // }

            for interaction in self.interactions.iter() {
                let mut cars = self.cars.iter();
                let send_obstacle = |obstacle: Obstacle| interaction.partner_lane << Add::InteractionObstacle(obstacle);
                
                if (current_tick + 1) % TRAFFIC_LOGIC_THROTTLING == interaction.partner_lane.instance_id as usize % TRAFFIC_LOGIC_THROTTLING {
                    match *interaction {
                        Interaction{start, partner_start, kind: Overlap{end, kind, ..}, ..} => {
                            match kind {
                                Parallel | Transfer => cars.skip_while(|car: &&LaneCar| *car.position + 2.0 * car.velocity < start)
                                                .take_while(|car: &&LaneCar| *car.position < end)
                                                .map(|car| car.as_obstacle.offset_by(-start + partner_start)
                                            ).foreach(send_obstacle),
                                Conflicting => if cars.any(|car: &LaneCar| *car.position + 2.0 * car.velocity > start && *car.position - 2.0 < end) {
                                    (send_obstacle)(Obstacle{position: OrderedFloat(partner_start), velocity: 0.0, max_velocity: 0.0})
                                }
                            }
                        }
                        Interaction{start, partner_start, kind: Previous, ..} =>
                            if let Some(next_car) = cars.find(|car| *car.position > start) {
                                (send_obstacle)(next_car.as_obstacle.offset_by(-start + partner_start))
                            },
                        Interaction{kind: Next{..}, ..} => {
                            //TODO: for looking backwards for merging lanes?
                        }
                    };
                }
            }

            if do_traffic {
                self.interaction_obstacles.clear();
            }

            Fate::Live
        }
    }}
}

impl Recipient<Tick> for TransferLane {
    fn receive(&mut self, msg: &Tick) -> Fate {match *msg{
        Tick{dt, current_tick} => {
            self.in_construction += dt * 400.0;

            let do_traffic = current_tick % TRAFFIC_LOGIC_THROTTLING == self.id().instance_id as usize % TRAFFIC_LOGIC_THROTTLING;

            if do_traffic {

                self.interaction_obstacles.sort_by_key(|obstacle| obstacle.position);
                let mut overlap_obstacles = self.interaction_obstacles.iter();
                let mut maybe_next_overlap_obstacle = overlap_obstacles.next();
                let mut maybe_previous_overlap_obstacle = None;

                for c in 0..self.cars.len() {
                    let (acceleration, is_dangerous) = {
                        let car = &self.cars[c];
                        
                        let next_obstacle = self.cars.get(c + 1).map_or(Obstacle::far_ahead(), |car| car.as_obstacle);
                        let previous_obstacle = if c > 0 {self.cars[c - 1].as_obstacle} else {Obstacle::far_behind()};

                        maybe_next_overlap_obstacle = maybe_next_overlap_obstacle.and_then(|obstacle| {
                            let mut following_obstacle = Some(obstacle);
                            while following_obstacle.is_some() && following_obstacle.unwrap().position < car.position {
                                maybe_previous_overlap_obstacle = Some(following_obstacle.unwrap());
                                following_obstacle = overlap_obstacles.next();
                            }
                            following_obstacle
                        });

                        let next_obstacle_acceleration = intelligent_acceleration(car, &next_obstacle)
                            .min(intelligent_acceleration(car, maybe_next_overlap_obstacle.unwrap_or(&Obstacle::far_ahead())));
                        let previous_obstacle_acceleration = intelligent_acceleration(&previous_obstacle, &car.as_obstacle)
                            .min(intelligent_acceleration(maybe_previous_overlap_obstacle.unwrap_or(&Obstacle::far_behind()), &car.as_obstacle));

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
                // smooth out arrival on other lane
                #[allow(float_cmp)]
                let arriving_soon = car.transfer_velocity.abs() > 0.1 && car.transfer_position.abs() > 0.5 && car.transfer_position.signum() == car.transfer_velocity.signum();
                if arriving_soon {
                    car.transfer_acceleration = -0.9 * car.transfer_velocity;
                }
            }

            if let (Some((left, left_start)), Some((right, right_start))) = (self.left, self.right) {
                let mut i = 0;
                loop {
                    let (should_remove, done) = if let Some(car) = self.cars.get(i) {
                        if car.transfer_position < -1.0 {
                            right << Add::Car(car.as_lane_car.offset_by(left_start), Some(self.id()));
                            (true, false)
                        } else if car.transfer_position > 1.0 {
                            left << Add::Car(car.as_lane_car.offset_by(right_start), Some(self.id()));
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

                if (current_tick + 1) % TRAFFIC_LOGIC_THROTTLING == left.instance_id as usize % TRAFFIC_LOGIC_THROTTLING {
                    for car in &self.cars {
                        if car.transfer_position < 0.3 || car.transfer_velocity > 0.0 {
                            left << Add::InteractionObstacle(car.as_obstacle.offset_by(left_start));
                        }
                    }
                }

                if (current_tick + 1) % TRAFFIC_LOGIC_THROTTLING == right.instance_id as usize % TRAFFIC_LOGIC_THROTTLING {
                    for car in &self.cars {
                        if car.transfer_position > -0.3 || car.transfer_velocity < 0.0 {
                            right << Add::InteractionObstacle(car.as_obstacle.offset_by(right_start));
                        }
                    }
                }
            }

            if do_traffic {
                self.interaction_obstacles.clear();
            }

            Fate::Live
        }
    }}
}

use self::planning::materialized_reality::BuildableRef;

#[derive(Copy, Clone)]
pub struct AdvertiseForConnectionAndReport(ID, BuildableRef);

#[derive(Compact, Clone)]
pub struct Connect{other_id: ID, other_path: CPath, reply_needed: bool, to_transfer: bool}

use self::planning::materialized_reality::ReportLaneBuilt;

impl Recipient<AdvertiseForConnectionAndReport> for Lane {
    fn receive(&mut self, msg: &AdvertiseForConnectionAndReport) -> Fate {match *msg{
        AdvertiseForConnectionAndReport(report_to, report_as) => {
            Swarm::<Lane>::all() << Connect{
                other_id: self.id(),
                other_path: self.path.clone(),
                reply_needed: true,
                to_transfer: false
            };
            Swarm::<TransferLane>::all() << Connect{
                other_id: self.id(),
                other_path: self.path.clone(),
                reply_needed: true,
                to_transfer: false
            };
            report_to << ReportLaneBuilt(self.id(), report_as);
            self::lane_rendering::on_build(self);
            self::pathfinding::on_build(self);
            Fate::Live
        }
    }}
}

impl Recipient<AdvertiseForConnectionAndReport> for TransferLane {
    fn receive(&mut self, msg: &AdvertiseForConnectionAndReport) -> Fate {match *msg{
        AdvertiseForConnectionAndReport(report_to, report_as) => {
            Swarm::<Lane>::all() << Connect{
                other_id: self.id(),
                other_path: self.path.clone(),
                reply_needed: true,
                to_transfer: true
            };
            report_to << ReportLaneBuilt(self.id(), report_as);
            self::lane_rendering::on_build_transfer(self);
            Fate::Live
        }
    }}
}

const CONNECTION_TOLERANCE: f32 = 0.1;

use fnv::FnvHashMap;
use ::std::cell::UnsafeCell;
thread_local! (
    static MEMOIZED_BANDS_OUTLINES: UnsafeCell<FnvHashMap<ID, (Band<CPath>, CPath)>> = UnsafeCell::new(FnvHashMap::default());
);

impl Recipient<Connect> for Lane {
    #[inline(never)]
    fn receive(&mut self, msg: &Connect) -> Fate {match *msg{
        Connect{other_id, ref other_path, reply_needed, to_transfer} => {
            if other_id == self.id() {return Fate::Live};

            if to_transfer {
                assert!(reply_needed, "transfer lanes should just want lane info on connect");
                other_id << Connect{
                    other_id: self.id(),
                    other_path: self.path.clone(),
                    reply_needed: false,
                    to_transfer: false
                };
                return Fate::Live;
            }

            if other_path.start().is_roughly_within(self.path.end(), CONNECTION_TOLERANCE) {
                self.interactions.push(Interaction{
                    partner_lane: other_id,
                    start: self.length,
                    partner_start: 0.0,
                    kind: InteractionKind::Next
                })
            }

            if other_path.end().is_roughly_within(self.path.start(), CONNECTION_TOLERANCE) {
                self.interactions.push(Interaction{
                    partner_lane: other_id,
                    start: 0.0,
                    partner_start: other_path.length(),
                    kind: InteractionKind::Previous
                })
            }

            MEMOIZED_BANDS_OUTLINES.with(|memoized_bands_outlines_cell| {
                let memoized_bands_outlines = unsafe{&mut *memoized_bands_outlines_cell.get()};
                let &(ref self_band, ref self_outline) = memoized_bands_outlines.entry(self.id()).or_insert_with(|| {
                    let band = Band::new(self.path.clone(), 4.0);
                    let outline = band.outline();
                    (band, outline)
                }) as &(Band<CPath>, CPath);

                let memoized_bands_outlines = unsafe{&mut *memoized_bands_outlines_cell.get()};
                let &(ref other_band, ref other_outline) = memoized_bands_outlines.entry(other_id).or_insert_with(|| {
                    let band = Band::new(other_path.clone(), 4.0);
                    let outline = band.outline();
                    (band, outline)
                }) as &(Band<CPath>, CPath);
                
                let intersections = (self_outline, other_outline).intersect();
                if intersections.len() >= 2 {
                    if let ::itertools::MinMaxResult::MinMax(
                        (entry_intersection, entry_distance),
                        (exit_intersection, exit_distance)
                    ) = intersections.iter().map(
                        |intersection| (intersection, self_band.outline_distance_to_path_distance(intersection.along_a))
                    ).minmax_by_key(|&(_, distance)| OrderedFloat(distance)) {
                        let other_entry_distance = other_band.outline_distance_to_path_distance(entry_intersection.along_b);
                        let other_exit_distance = other_band.outline_distance_to_path_distance(exit_intersection.along_b);

                        let overlap_kind = if other_path.direction_along(other_entry_distance)
                            .is_roughly_within(self.path.direction_along(entry_distance), 0.1)
                        || other_path.direction_along(other_exit_distance)
                            .is_roughly_within(self.path.direction_along(exit_distance), 0.1) {
                                OverlapKind::Parallel
                            } else {
                                OverlapKind::Conflicting
                            };

                        self.interactions.push(Interaction{
                            partner_lane: other_id,
                            start: entry_distance,
                            partner_start: other_entry_distance.min(other_exit_distance),
                            kind: InteractionKind::Overlap{
                                end: exit_distance,
                                partner_end: other_exit_distance.max(other_entry_distance),
                                kind: overlap_kind
                            }
                        });
                    } else {panic!("both entry and exit should exist")}
                }


                if reply_needed {
                    other_id << Connect{
                        other_id: self.id(),
                        other_path: self.path.clone(),
                        reply_needed: false,
                        to_transfer: false
                    };
                }
                Fate::Live
            })
        }
    }}
}

impl Recipient<Connect> for TransferLane {
    #[inline(never)]
    fn receive(&mut self, msg: &Connect) -> Fate {match *msg{
        Connect{other_id, ref other_path, ..} => {
            if self.path.segments().iter().all(|segment|
                other_path.segments().iter().any(|other_segment|
                    segment.start().is_roughly_within(other_segment.start(), 6.0)
                    && segment.start_direction().is_roughly_within(other_segment.start_direction(), 0.1)
                    && segment.end().is_roughly_within(other_segment.end(), 6.0)
                    && segment.end_direction().is_roughly_within(other_segment.end_direction(), 0.1)
                )
            ) {
                let self_start_on_other_distance = other_path.project(self.path.start())
                    .expect("start should be on neighboring lane");
                let self_start_on_other = other_path.along(self_start_on_other_distance);
                let is_right_of = (self.path.start() - self_start_on_other)
                    .dot(&self.path.start_direction().orthogonal()) > 0.0;

                if is_right_of {
                    self.right = Some((other_id, self_start_on_other_distance));
                    other_id << AddTransferLaneInteraction(Interaction{
                        partner_lane: self.id(),
                        start: self_start_on_other_distance,
                        partner_start: 0.0,
                        kind: InteractionKind::Overlap{
                            end: self_start_on_other_distance + self.length,
                            partner_end: self.length,
                            kind: OverlapKind::Transfer
                        }
                    })
                } else {
                    self.left = Some((other_id, self_start_on_other_distance));
                    other_id << AddTransferLaneInteraction(Interaction{
                        partner_lane: self.id(),
                        start: self_start_on_other_distance,
                        partner_start: 0.0,
                        kind: InteractionKind::Overlap{
                            end: self_start_on_other_distance + self.length,
                            partner_end: self.length,
                            kind: OverlapKind::Transfer
                        }
                    })
                }
            }
            Fate::Live
        }
    }}
}

#[derive(Copy, Clone)]
pub struct AddTransferLaneInteraction(Interaction);

impl Recipient<AddTransferLaneInteraction> for Lane {
    fn receive(&mut self, msg: &AddTransferLaneInteraction) -> Fate {match *msg{
        AddTransferLaneInteraction(interaction) => {
            if !self.interactions.iter().any(|existing| existing.partner_lane == interaction.partner_lane) {
                self.interactions.push(interaction);
            }
            Fate::Live
        }
    }}
}

#[derive(Copy, Clone)]
pub struct Disconnect{other_id: ID}

impl Recipient<Disconnect> for Lane {
    fn receive(&mut self, msg: &Disconnect) -> Fate {match *msg{
        Disconnect{other_id} => {
            // TODO: use retain
            self.interactions = self.interactions.iter().filter(|interaction|
                interaction.partner_lane != other_id
            ).cloned().collect();
            Fate::Live
        }
    }}
}

impl Recipient<Disconnect> for TransferLane {
    fn receive(&mut self, msg: &Disconnect) -> Fate {match *msg{
        Disconnect{other_id} => {
            let mut something_changed = false;
            self.left = self.left.and_then(|(left_id, left_start)|
                if left_id == other_id {
                    something_changed = true;
                    None
                } else {Some((left_id, left_start))}
            );
            self.right = self.right.and_then(|(right_id, right_start)|
                if right_id == other_id {
                    something_changed = true;
                    None
                } else {Some((right_id, right_start))}
            );
            if !something_changed {panic!("Tried to disconnect a non-connected lane")}
            Fate::Live
        }
    }}
}

#[derive(Copy, Clone)]
pub struct Unbuild;

impl Recipient<Unbuild> for Lane{
    fn receive(&mut self, _msg: &Unbuild) -> Fate {
        Swarm::<Lane>::all() << Disconnect{other_id: self.id()}; 
        self::lane_rendering::on_unbuild(self);
        MEMOIZED_BANDS_OUTLINES.with(|memoized_bands_outlines_cell| {
                let memoized_bands_outlines = unsafe{&mut *memoized_bands_outlines_cell.get()};
                memoized_bands_outlines.remove(&self.id())
        });
        Fate::Die
    }
}

impl Recipient<Unbuild> for TransferLane{
    fn receive(&mut self, _msg: &Unbuild) -> Fate {
        if let Some((left_id, _)) = self.left {
            left_id << Disconnect{other_id: self.id()}; 
        }
        if let Some((right_id, _)) = self.right {
            right_id << Disconnect{other_id: self.id()}; 
        }
        self::lane_rendering::on_unbuild_transfer(self);
        Fate::Die
    }
}

pub fn setup(system: &mut ActorSystem) {
    system.add_individual(Swarm::<Lane>::new());
    system.add_inbox::<CreateWith<Lane, AdvertiseForConnectionAndReport>, Swarm<Lane>>();
    system.add_inbox::<Add, Swarm<Lane>>();
    system.add_inbox::<Tick, Swarm<Lane>>();
    system.add_inbox::<Connect, Swarm<Lane>>();
    system.add_inbox::<AddTransferLaneInteraction, Swarm<Lane>>();
    system.add_inbox::<Disconnect, Swarm<Lane>>();
    system.add_inbox::<Unbuild, Swarm<Lane>>();

    system.add_individual(Swarm::<TransferLane>::new());
    system.add_inbox::<CreateWith<TransferLane, AdvertiseForConnectionAndReport>, Swarm<TransferLane>>();
    system.add_inbox::<Add, Swarm<TransferLane>>();
    system.add_inbox::<Tick, Swarm<TransferLane>>();
    system.add_inbox::<Connect, Swarm<TransferLane>>();
    system.add_inbox::<Disconnect, Swarm<TransferLane>>();
    system.add_inbox::<Unbuild, Swarm<TransferLane>>();

    self::pathfinding::setup(system);
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
    acceleration: f32,
    destination: pathfinding::Destination,
    next_hop_interaction: u8
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

#[derive(Copy, Clone, Debug)]
struct Interaction {
    partner_lane: ID,
    start: f32,
    partner_start: f32,
    kind: InteractionKind
}

#[derive(Copy, Clone, Debug)]
enum InteractionKind{
    Overlap{
        end: f32,
        partner_end: f32,
        kind: OverlapKind
    },
    Next,
    Previous
}
use self::InteractionKind::{Overlap, Next, Previous};

#[derive(Copy, Clone, Debug)]
enum OverlapKind{Parallel, Transfer, Conflicting}
use self::OverlapKind::{Parallel, Transfer, Conflicting};