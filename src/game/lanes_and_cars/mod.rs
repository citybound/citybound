pub mod lane_rendering;
pub mod lane_thing_collector;
pub mod planning;
pub mod pathfinding;
mod intelligent_acceleration;
use self::intelligent_acceleration::{intelligent_acceleration};
use core::geometry::{CPath};
use kay::{ID, Actor, CVec, Swarm, CreateWith, Recipient, ActorSystem, Fate};
use descartes::{N, P2, FiniteCurve, RoughlyComparable, Band, Intersect, Curve, Dot, WithUniqueOrthogonal};
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
    obstacles: CVec<(Obstacle, ID)>,
    cars: CVec<LaneCar>,
    in_construction: f32,
    on_intersection: bool,
    timings: CVec<bool>,
    green: bool,
    pathfinding_info: pathfinding::PathfindingInfo,
    hovered: bool,
    unbuilding_for: Option<ID>,
    disconnects_remaining: u8
}

impl Lane {
    pub fn new(path: CPath, on_intersection: bool, timings: CVec<bool>) -> Self {
        Lane {
            _id: ID::invalid(),
            length: path.length(),
            path: path,
            interactions: CVec::new(),
            obstacles: CVec::new(),
            cars: CVec::new(),
            in_construction: 0.0,
            on_intersection: on_intersection,
            timings: timings,
            green: false,
            pathfinding_info: pathfinding::PathfindingInfo::default(),
            hovered: false,
            unbuilding_for: None,
            disconnects_remaining: 0
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
    left_obstacles: CVec<Obstacle>,
    right_obstacles: CVec<Obstacle>,
    cars: CVec<TransferringLaneCar>,
    in_construction: f32,
    unbuilding_for: Option<ID>,
    disconnects_remaining: u8
}

impl TransferLane {
    fn new(path: CPath) -> TransferLane {
        TransferLane{
            _id: ID::invalid(),
            length: path.length(),
            path: path,
            left: None,
            right: None,
            left_obstacles: CVec::new(),
            right_obstacles: CVec::new(),
            cars: CVec::new(),
            in_construction: 0.0,
            unbuilding_for: None,
            disconnects_remaining: 0
        }
    }

    fn other_side(&self, side: ID) -> ID {
        if side == self.left.expect("should have a left lane").0 {
            self.right.expect("should have a right lane").0
        } else {
            self.left.expect("should have a left lane").0
        }
    }
}

#[derive(Copy, Clone)]
struct AddCar{car: LaneCar, from: Option<ID>}

#[derive(Compact, Clone)]
struct AddObstacles{obstacles: CVec<Obstacle>, from: ID}

use self::pathfinding::RoutingInfo;

impl Recipient<AddCar> for Lane {
    fn receive(&mut self, msg: &AddCar) -> Fate {match *msg{
        AddCar{car, ..} => {
            let maybe_next_hop_interaction = self.pathfinding_info.routes.get(car.destination)
            .or(self.pathfinding_info.routes.get(car.destination.landmark_destination()))
            .map(|&RoutingInfo{outgoing_idx, ..}| {
                outgoing_idx as usize
            });

            if let Some(next_hop_interaction) = maybe_next_hop_interaction {
                let routed_car = LaneCar{
                    next_hop_interaction: next_hop_interaction as u8,
                    as_obstacle: if *car.as_obstacle.position < 0.0 {
                        car.as_obstacle.offset_by(-*car.as_obstacle.position).offset_by(
                            self.cars.get(0).map(|last_car| *last_car.position).unwrap_or(self.length / 2.0) - 6.0
                        )
                    } else {car.as_obstacle},
                    .. car
                };

                if *routed_car.position < 0.0 {
                    // TODO: cancel trip
                    return Fate::Live;
                } 

                // TODO: optimize using BinaryHeap?
                let maybe_next_car_position = self.cars.iter().position(|other_car| other_car.as_obstacle.position > car.as_obstacle.position);
                match maybe_next_car_position {
                    Some(next_car_position) => self.cars.insert(next_car_position, routed_car),
                    None => self.cars.push(routed_car)
                }
            } else {
                println!("NO ROUTE!");
            }
            Fate::Live
        }
    }}
}

impl Recipient<AddCar> for TransferLane {
    fn receive(&mut self, msg: &AddCar) -> Fate {match *msg{
        AddCar{car, from: Some(from)} => {
            let side_multiplier = if from == self.left.expect("should have a left lane").0 {-1.0} else {1.0};
            self.cars.push(TransferringLaneCar{
                as_lane_car: car,
                transfer_position: 1.0 * side_multiplier,
                transfer_velocity: 0.0,
                transfer_acceleration: 0.3 * -side_multiplier,
                cancelling: false
            });
            // TODO: optimize using BinaryHeap?
            self.cars.sort_by_key(|car| car.as_obstacle.position);
            Fate::Live
        },
        AddCar{from: None, ..} => panic!("car has to come from somewhere on a transfer lane"),
    }}
}

impl Recipient<AddObstacles> for Lane {
    fn receive(&mut self, msg: &AddObstacles) -> Fate {match *msg{
        AddObstacles{ref obstacles, from} => {
            self.obstacles.retain(|&(_, received_from)| received_from != from);
            self.obstacles.extend(obstacles.iter().map(|obstacle| (*obstacle, from)));
            Fate::Live
        }
    }}
}

impl Recipient<AddObstacles> for TransferLane {
    fn receive(&mut self, msg: &AddObstacles) -> Fate {match *msg{
        AddObstacles{ref obstacles, from} => {
            if let (Some((left_id, _)), Some(_)) = (self.left, self.right) {
                let target_obstacles = if left_id == from {
                    &mut self.left_obstacles
                } else {&mut self.right_obstacles};
                *target_obstacles = obstacles.clone();
            } else {
                println!("transfer lane not connected for obstacles yet");
            }
            Fate::Live
        }
    }}
}

use core::simulation::Tick;

const TRAFFIC_LOGIC_THROTTLING : usize = 30;
const PATHFINDING_THROTTLING : usize = 10;

#[derive(Copy, Clone)]
pub struct SignalChanged{
    from: ID,
    green: bool
}

impl Recipient<Tick> for Lane {
    fn receive(&mut self, msg: &Tick) -> Fate {match *msg{
        Tick{dt, current_tick} => {
            self.in_construction += dt * 400.0;

            let old_green = self.green;
            self.green = if self.timings.is_empty() {true} else {self.timings[(current_tick / 25) % self.timings.len()]};

            if old_green != self.green {
                for interaction in &self.interactions {
                    if let Interaction{kind: InteractionKind::Previous{..}, partner_lane, ..} = *interaction {
                        partner_lane << SignalChanged{from: self.id(), green: self.green}
                    }
                }
            }

            if current_tick % PATHFINDING_THROTTLING == self.id().instance_id as usize % PATHFINDING_THROTTLING {
                self::pathfinding::tick(self);
            }

            let do_traffic = current_tick % TRAFFIC_LOGIC_THROTTLING == self.id().instance_id as usize % TRAFFIC_LOGIC_THROTTLING;

            if do_traffic {
                // TODO: optimize using BinaryHeap?
                self.obstacles.sort_by_key(|&(ref obstacle, _id)| obstacle.position);

                let mut obstacles = self.obstacles.iter().map(|&(ref obstacle, _id)| obstacle);
                let mut maybe_next_obstacle = obstacles.next();

                for c in 0..self.cars.len() {
                    let next_obstacle = self.cars.get(c + 1).map_or(Obstacle::far_ahead(), |car| car.as_obstacle);
                    let car = &mut self.cars[c];
                    let next_car_acceleration = intelligent_acceleration(car, &next_obstacle, 2.0);
                    
                    maybe_next_obstacle = maybe_next_obstacle.and_then(|obstacle| {
                        let mut following_obstacle = Some(obstacle);
                        while following_obstacle.is_some() && *following_obstacle.unwrap().position < *car.position + 0.1 {
                            following_obstacle = obstacles.next();
                        }
                        following_obstacle
                    });
                    
                    let next_obstacle_acceleration = if let Some(next_obstacle) = maybe_next_obstacle {
                        intelligent_acceleration(car, next_obstacle, 4.0)
                    } else {INFINITY};

                    car.acceleration = next_car_acceleration.min(next_obstacle_acceleration);

                    if let Interaction{start, kind: InteractionKind::Next{green}, ..} = self.interactions[car.next_hop_interaction as usize] {
                        if !green {
                            car.acceleration = car.acceleration.min(
                                intelligent_acceleration(car, &Obstacle{
                                    position: OrderedFloat(start + 2.0), velocity: 0.0, max_velocity: 0.0
                                }, 2.0)
                            )
                        }
                    }
                }
            }

            for car in &mut self.cars {
                *car.position += dt * car.velocity;
                car.velocity = (car.velocity + dt * car.acceleration).min(car.max_velocity).max(0.0);
            }

            for &mut (ref mut obstacle, _id) in &mut self.obstacles {
                *obstacle.position += dt * obstacle.velocity;
            }

            if self.cars.len() > 1 {
                for i in (0..self.cars.len() - 1).rev() {
                    self.cars[i].position = OrderedFloat((*self.cars[i].position).min(*self.cars[i + 1].position));
                }
            }

            loop {
                let maybe_switch_car = self.cars.iter().enumerate().rev().filter_map(|(i, &car)| {
                    let interaction = self.interactions[car.next_hop_interaction as usize];

                    match interaction.kind {
                        InteractionKind::Overlap{end, kind: OverlapKind::Transfer, ..} => if *car.position > interaction.start && *car.position > end - 300.0 {
                            Some((i, interaction.partner_lane, interaction.start, interaction.partner_start))
                        }else {None},
                        _ => if *car.position > interaction.start {
                            Some((i, interaction.partner_lane, interaction.start, interaction.partner_start))
                        } else {None}
                    }
                }).next();

                if let Some((idx_to_remove, next_lane, start, partner_start)) = maybe_switch_car {
                    let car = self.cars.remove(idx_to_remove);
                    if self.id() != car.destination.node {
                        next_lane << AddCar{car: car.offset_by(partner_start - start), from: Some(self.id())};
                    }
                } else {
                    break;
                }
            }

            // ASSUMPTION: only one interaction per Lane/Lane pair
            for interaction in self.interactions.iter() {
                let mut cars = self.cars.iter();
                
                if (current_tick + 1) % TRAFFIC_LOGIC_THROTTLING == interaction.partner_lane.instance_id as usize % TRAFFIC_LOGIC_THROTTLING {
                    let maybe_obstacles : Option<CVec<_>> = match *interaction {
                        Interaction{partner_lane, start, partner_start, kind: Overlap{end, kind, ..}, ..} =>
                            Some(match kind {
                                Parallel =>
                                    cars.skip_while(|car: &&LaneCar| *car.position + 2.0 * car.velocity < start)
                                        .take_while(|car: &&LaneCar| *car.position < end)
                                        .map(|car| car.as_obstacle.offset_by(-start + partner_start))
                                        .collect(),
                                Transfer =>
                                    cars.skip_while(|car: &&LaneCar| *car.position + 2.0 * car.velocity < start)
                                        .map(|car| car.as_obstacle.offset_by(-start + partner_start))
                                        .chain(self.obstacles.iter().filter_map(|&(obstacle, id)|
                                            if id != partner_lane && *obstacle.position + 2.0 * obstacle.velocity > start {
                                                Some(obstacle.offset_by(-start + partner_start))
                                            } else {None}
                                        ))
                                        .collect(),
                                Conflicting =>
                                    if cars.any(|car: &LaneCar|
                                        *car.position + 2.0 * car.velocity > start && *car.position - 2.0 < end
                                    ) {
                                        vec![Obstacle{position: OrderedFloat(partner_start), velocity: 0.0, max_velocity: 0.0}].into()
                                    } else {
                                        CVec::new()
                                    }
                            }),
                        Interaction{start, partner_start, kind: Previous, ..} =>
                            Some(cars.map(|car| &car.as_obstacle)
                                .chain(self.obstacles.iter().map(|&(ref obstacle, _id)| obstacle))
                                .find(|car| *car.position >= start - 2.0)
                                .map(|first_car| first_car.offset_by(-start + partner_start))
                                .into_iter().collect()),
                        Interaction{kind: Next{..}, ..} => {
                            None
                            //TODO: for looking backwards for merging lanes?
                        }
                    };

                    if let Some(obstacles) = maybe_obstacles {
                        interaction.partner_lane << AddObstacles{obstacles: obstacles, from: self.id()}
                    }
                }
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
                // TODO: optimize using BinaryHeap?
                self.left_obstacles.sort_by_key(|obstacle| obstacle.position);
                self.right_obstacles.sort_by_key(|obstacle| obstacle.position);

                for c in 0..self.cars.len() {
                    let (acceleration, dangerous) = {
                        let car = &self.cars[c];
                        let next_car = self.cars.iter().find(|other_car|
                            *other_car.position > *car.position
                        ).map(|other_car| &other_car.as_obstacle);

                        let maybe_next_left_obstacle = if car.transfer_position < 0.3 || car.transfer_acceleration < 0.0 {
                            self.left_obstacles.iter().find(|obstacle| *obstacle.position + 5.0 > *car.position)
                        } else {None};

                        let maybe_next_right_obstacle = if car.transfer_position > -0.3 || car.transfer_acceleration > 0.0 {
                            self.right_obstacles.iter().find(|obstacle| *obstacle.position + 5.0 > *car.position)
                        } else {None};

                        // TODO: sometimes cars get stuck when on top of each other or merge into standing queues happily
                        let next_obstacle_acceleration = *next_car.into_iter().chain(maybe_next_left_obstacle)
                            .chain(maybe_next_right_obstacle).chain(&[Obstacle::far_ahead()]).map(|obstacle| {
                                let corrected_obstacle = if obstacle.position < car.position {
                                    Obstacle{
                                        position: OrderedFloat(*car.position + 18.0),
                                        velocity: 0.0,
                                        max_velocity: 0.0
                                    }
                                } else {*obstacle};
                                OrderedFloat(intelligent_acceleration(car, &corrected_obstacle, 1.0))
                            }).min().unwrap();

                        let transfer_before_end_velocity = (self.length + 1.0 - *car.position) / 1.5;
                        let transfer_before_end_acceleration = transfer_before_end_velocity - car.velocity;

                        let dangerous = car.velocity > 5.0 && next_obstacle_acceleration < -7.0;

                        (next_obstacle_acceleration.min(transfer_before_end_acceleration), dangerous)
                    };

                    let car = &mut self.cars[c];
                    car.acceleration = acceleration;

                    if dangerous && !car.cancelling {
                        car.transfer_acceleration = -car.transfer_acceleration;
                        car.cancelling = true;
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
            }

            for obstacle in self.left_obstacles.iter_mut().chain(self.right_obstacles.iter_mut()) {
                *obstacle.position += dt * obstacle.velocity;
            }

            if self.cars.len() > 1 {
                for i in (0..self.cars.len() - 1).rev() {
                    if self.cars[i].position > self.cars[i + 1].position {
                        self.cars.swap(i, i + 1);
                    }
                }
            }

            if let (Some((left, left_start)), Some((right, right_start))) = (self.left, self.right) {
                let mut i = 0;
                loop {
                    let (should_remove, done) = if let Some(car) = self.cars.get(i) {
                        if car.transfer_position > 1.0 || (*car.position > self.length && car.transfer_acceleration > 0.0) {
                            right << AddCar{car: car.as_lane_car.offset_by(left_start), from: Some(self.id())};
                            (true, false)
                        } else if car.transfer_position < -1.0 || (*car.position > self.length && car.transfer_acceleration <= 0.0)  {
                            left << AddCar{car: car.as_lane_car.offset_by(right_start), from: Some(self.id())};
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
                    let obstacles = self.cars.iter().filter_map(|car|
                        if car.transfer_position < 0.3 || car.transfer_acceleration < 0.0 {
                            Some(car.as_obstacle.offset_by(left_start))
                        } else {None}
                    ).collect();
                    left << AddObstacles{obstacles: obstacles, from: self.id()};
                }

                if (current_tick + 1) % TRAFFIC_LOGIC_THROTTLING == right.instance_id as usize % TRAFFIC_LOGIC_THROTTLING {
                    let obstacles = self.cars.iter().filter_map(|car|
                        if car.transfer_position > -0.3 || car.transfer_acceleration > 0.0 {
                            Some(car.as_obstacle.offset_by(right_start))
                        } else {None}
                    ).collect();
                    right << AddObstacles{obstacles: obstacles, from: self.id()};
                }
            }

            Fate::Live
        }
    }}
}

impl Recipient<SignalChanged> for Lane {
    fn receive(&mut self, msg: &SignalChanged) -> Fate {match *msg{
        SignalChanged{from, green} => {
            if let Some(interaction) = self.interactions.iter_mut().find(|interaction|
                match **interaction {
                    Interaction{partner_lane, kind: InteractionKind::Next{..}, ..} => partner_lane == from,
                    _ => false
                }
            ) {
                interaction.kind = InteractionKind::Next{green: green}
            } else {
                println!("Lane doesn't know about next lane yet");
            }
            Fate::Live
        }
    }}
}

use self::planning::materialized_reality::BuildableRef;

#[derive(Copy, Clone)]
pub struct AdvertiseToTransferAndReport(ID, BuildableRef);

use self::planning::materialized_reality::ReportLaneBuilt;

impl Recipient<AdvertiseToTransferAndReport> for Lane {
    fn receive(&mut self, msg: &AdvertiseToTransferAndReport) -> Fate {match *msg{
        AdvertiseToTransferAndReport(report_to, report_as) => {
            Swarm::<Lane>::all() << Connect{
                other_id: self.id(),
                other_start: self.path.start(),
                other_end: self.path.end(),
                other_length: self.path.length(),
                reply_needed: true
            };
            Swarm::<TransferLane>::all() << ConnectTransferToNormal{
                other_id: self.id(),
                other_path: self.path.clone()
            };
            report_to << ReportLaneBuilt(self.id(), report_as);
            self::lane_rendering::on_build(self);
            self::pathfinding::on_build(self);
            Fate::Live
        }
    }}
}

impl Recipient<AdvertiseToTransferAndReport> for TransferLane {
    fn receive(&mut self, msg: &AdvertiseToTransferAndReport) -> Fate {match *msg{
        AdvertiseToTransferAndReport(report_to, report_as) => {
            Swarm::<Lane>::all() << ConnectToTransfer{
                other_id: self.id()
            };
            report_to << ReportLaneBuilt(self.id(), report_as);
            self::lane_rendering::on_build_transfer(self);
            Fate::Live
        }
    }}
}

#[derive(Compact, Clone)]
pub struct AdvertiseForOverlaps{lanes: CVec<ID>}

impl Recipient<AdvertiseForOverlaps> for Lane {
    fn receive(&mut self, msg: &AdvertiseForOverlaps) -> Fate {match *msg{
        AdvertiseForOverlaps{ref lanes} => {
            for &lane in lanes.iter() {
                lane << ConnectOverlaps{
                    other_id: self.id(),
                    other_path: self.path.clone(),
                    reply_needed: true
                };
            }
            Fate::Live
        }
    }}
}

const CONNECTION_TOLERANCE: f32 = 0.1;

#[derive(Copy, Clone)]
pub struct Connect{other_id: ID, other_start: P2, other_end: P2, other_length: N, reply_needed: bool}

impl Recipient<Connect> for Lane {
    #[inline(never)]
    fn receive(&mut self, msg: &Connect) -> Fate {match *msg{
        Connect{other_id, other_start, other_end, other_length, reply_needed} => {
            if other_id == self.id() {return Fate::Live};

            let mut connected = false;

            if other_start.is_roughly_within(self.path.end(), CONNECTION_TOLERANCE) {
                connected = true;

                if !self.interactions.iter().any(|interaction| match *interaction {
                    Interaction{partner_lane, kind: InteractionKind::Next{..}, ..} => partner_lane == other_id,
                    _ => false
                }) {
                    self.interactions.push(Interaction{
                        partner_lane: other_id,
                        start: self.length,
                        partner_start: 0.0,
                        kind: InteractionKind::Next{green: false}
                    });
                }

                pathfinding::on_connect(self);
            }

            if other_end.is_roughly_within(self.path.start(), CONNECTION_TOLERANCE) {
                connected = true;

                if !self.interactions.iter().any(|interaction| match *interaction {
                    Interaction{partner_lane, kind: InteractionKind::Previous{..}, ..} => partner_lane == other_id,
                    _ => false
                }) {
                    self.interactions.push(Interaction{
                        partner_lane: other_id,
                        start: 0.0,
                        partner_start: other_length,
                        kind: InteractionKind::Previous
                    });
                }

                pathfinding::on_connect(self);
            }

            if reply_needed && connected {
                other_id << Connect{
                    other_id: self.id(),
                    other_start: self.path.start(),
                    other_end: self.path.end(),
                    other_length: self.path.length(),
                    reply_needed: false
                };
            }

            Fate::Live
        }
    }}
}

use fnv::FnvHashMap;
use ::std::cell::UnsafeCell;
thread_local! (
    static MEMOIZED_BANDS_OUTLINES: UnsafeCell<FnvHashMap<ID, (Band<CPath>, CPath)>> = UnsafeCell::new(FnvHashMap::default());
);

#[derive(Compact, Clone)]
pub struct ConnectOverlaps{other_id: ID, other_path: CPath, reply_needed: bool}

impl Recipient<ConnectOverlaps> for Lane {
    fn receive(&mut self, msg: &ConnectOverlaps) -> Fate {match *msg{
        ConnectOverlaps{other_id, ref other_path, reply_needed} => {
            MEMOIZED_BANDS_OUTLINES.with(|memoized_bands_outlines_cell| {
                let memoized_bands_outlines = unsafe{&mut *memoized_bands_outlines_cell.get()};
                let &(ref self_band, ref self_outline) = memoized_bands_outlines.entry(self.id()).or_insert_with(|| {
                    let band = Band::new(self.path.clone(), 4.5);
                    let outline = band.outline();
                    (band, outline)
                }) as &(Band<CPath>, CPath);

                let memoized_bands_outlines = unsafe{&mut *memoized_bands_outlines_cell.get()};
                let &(ref other_band, ref other_outline) = memoized_bands_outlines.entry(other_id).or_insert_with(|| {
                    let band = Band::new(other_path.clone(), 4.5);
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
                                //::core::geometry::add_debug_path(self.path.subsection(entry_distance, exit_distance).unwrap(), [1.0, 0.5, 0.0], 0.3);
                                OverlapKind::Parallel
                            } else {
                                //::core::geometry::add_debug_path(self.path.subsection(entry_distance, exit_distance).unwrap(), [1.0, 0.0, 0.0], 0.3);
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
                    other_id << ConnectOverlaps{
                        other_id: self.id(),
                        other_path: self.path.clone(),
                        reply_needed: false
                    };
                }
                Fate::Live
            })
        }
    }}
}

#[derive(Compact, Clone)]
pub struct ConnectToTransfer{other_id: ID}

impl Recipient<ConnectToTransfer> for Lane {
    fn receive(&mut self, msg: &ConnectToTransfer) -> Fate {match *msg{
        ConnectToTransfer{other_id} => {
            other_id << ConnectTransferToNormal{
                other_id: self.id(),
                other_path: self.path.clone(),
            };
            Fate::Live
        }
    }}
}

#[derive(Compact, Clone)]
pub struct ConnectTransferToNormal{other_id: ID, other_path: CPath}

impl Recipient<ConnectTransferToNormal> for TransferLane {
    #[inline(never)]
    fn receive(&mut self, msg: &ConnectTransferToNormal) -> Fate {match *msg{
        ConnectTransferToNormal{other_id, ref other_path} => {
            let projections = (other_path.project(self.path.start()), other_path.project(self.path.end()));
            if let (Some(self_start_on_other_distance), Some(self_end_on_other_distance)) = projections {
                if self_start_on_other_distance < self_end_on_other_distance
                && self_end_on_other_distance - self_start_on_other_distance > 6.0 {
                    let self_start_on_other = other_path.along(self_start_on_other_distance);
                    let self_end_on_other = other_path.along(self_end_on_other_distance);

                    if self_start_on_other.is_roughly_within(self.path.start(), 3.0)
                    && self_end_on_other.is_roughly_within(self.path.end(), 3.0) {
                        other_id << AddTransferLaneInteraction(Interaction{
                            partner_lane: self.id(),
                            start: self_start_on_other_distance,
                            partner_start: 0.0,
                            kind: InteractionKind::Overlap{
                                end: self_start_on_other_distance + self.length,
                                partner_end: self.length,
                                kind: OverlapKind::Transfer
                            }
                        });
                        
                        let other_is_right = (self_start_on_other - self.path.start())
                            .dot(&self.path.start_direction().orthogonal()) > 0.0;

                        if other_is_right {
                            self.right = Some((other_id, self_start_on_other_distance));
                        } else {
                            self.left = Some((other_id, self_start_on_other_distance));
                        }
                    }
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
                pathfinding::on_connect(self);
            }
            Fate::Live
        }
    }}
}

#[derive(Copy, Clone)]
pub struct Disconnect{other_id: ID}
#[derive(Copy, Clone)]
pub struct ConfirmDisconnect;

impl Recipient<Disconnect> for Lane {
    fn receive(&mut self, msg: &Disconnect) -> Fate {match *msg{
        Disconnect{other_id} => {
            let interaction_indices_to_remove = self.interactions.iter().enumerate().filter_map(
                |(i, interaction)| if interaction.partner_lane == other_id {
                    Some(i)
                } else {None}
            ).collect::<Vec<_>>();
            // TODO: Cancel trip
            self.cars.retain(|car| !interaction_indices_to_remove.contains(&(car.next_hop_interaction as usize)));
            for idx in interaction_indices_to_remove.into_iter().rev() {
                self.interactions.remove(idx);
            }
            other_id << ConfirmDisconnect;
            Fate::Live
        }
    }}
}

impl Recipient<Disconnect> for TransferLane {
    fn receive(&mut self, msg: &Disconnect) -> Fate {match *msg{
        Disconnect{other_id} => {
            self.left = self.left.and_then(|(left_id, left_start)|
                if left_id == other_id {None} else {Some((left_id, left_start))}
            );
            self.right = self.right.and_then(|(right_id, right_start)|
                if right_id == other_id {None} else {Some((right_id, right_start))}
            );
            other_id << ConfirmDisconnect;
            Fate::Live
        }
    }}
}

#[derive(Copy, Clone)]
pub struct Unbuild{pub report_to: ID}
use self::planning::materialized_reality::ReportLaneUnbuilt;

impl Recipient<Unbuild> for Lane{
    fn receive(&mut self, msg: &Unbuild) -> Fate {match *msg {
        Unbuild{report_to} => {
            let mut disconnects_remaining = 0;
            for id in self.interactions.iter().map(|interaction| interaction.partner_lane).unique() {
                id << Disconnect{other_id: self.id()};
                disconnects_remaining += 1;
            }
            self::lane_rendering::on_unbuild(self);
            MEMOIZED_BANDS_OUTLINES.with(|memoized_bands_outlines_cell| {
                let memoized_bands_outlines = unsafe{&mut *memoized_bands_outlines_cell.get()};
                memoized_bands_outlines.remove(&self.id())
            });
            if disconnects_remaining == 0 {
                report_to << ReportLaneUnbuilt(Some(self.id()));
                Fate::Die
            } else {
                self.disconnects_remaining = disconnects_remaining;
                self.unbuilding_for = Some(report_to);
                Fate::Live
            }
        }
    }}
}

impl Recipient<Unbuild> for TransferLane{
    fn receive(&mut self, msg: &Unbuild) -> Fate {match *msg{
        Unbuild{report_to} => {
            if let Some((left_id, _)) = self.left {
                left_id << Disconnect{other_id: self.id()}; 
            }
            if let Some((right_id, _)) = self.right {
                right_id << Disconnect{other_id: self.id()}; 
            }
            self::lane_rendering::on_unbuild_transfer(self);
            if self.left.is_none() && self.right.is_none() {
                report_to << ReportLaneUnbuilt(Some(self.id()));
                Fate::Die
            } else {
                self.disconnects_remaining = self.left.into_iter().chain(self.right).count() as u8;
                self.unbuilding_for = Some(report_to);
                Fate::Live
            }
        }
    }}
}

impl Recipient<ConfirmDisconnect> for Lane {
    fn receive(&mut self, _msg: &ConfirmDisconnect) -> Fate {
        self.disconnects_remaining -= 1;
        if self.disconnects_remaining == 0 {
            self.unbuilding_for.expect("should be unbuilding") << ReportLaneUnbuilt(Some(self.id()));
            Fate::Die
        } else {Fate::Live}
    }
}

impl Recipient<ConfirmDisconnect> for TransferLane {
    fn receive(&mut self, _msg: &ConfirmDisconnect) -> Fate {
        self.disconnects_remaining -= 1;
        if self.disconnects_remaining == 0 {
            self.unbuilding_for.expect("should be unbuilding") << ReportLaneUnbuilt(Some(self.id()));
            Fate::Die
        } else {Fate::Live}
    }
}

pub fn setup(system: &mut ActorSystem) {
    system.add_individual(Swarm::<Lane>::new());
    system.add_inbox::<CreateWith<Lane, AdvertiseToTransferAndReport>, Swarm<Lane>>();
    system.add_inbox::<AdvertiseForOverlaps, Swarm<Lane>>();
    system.add_inbox::<AddCar, Swarm<Lane>>();
    system.add_inbox::<AddObstacles, Swarm<Lane>>();
    system.add_inbox::<Tick, Swarm<Lane>>();
    system.add_inbox::<SignalChanged, Swarm<Lane>>();
    system.add_inbox::<Connect, Swarm<Lane>>();
    system.add_inbox::<ConnectToTransfer, Swarm<Lane>>();
    system.add_inbox::<ConnectOverlaps, Swarm<Lane>>();
    system.add_inbox::<AddTransferLaneInteraction, Swarm<Lane>>();
    system.add_inbox::<Disconnect, Swarm<Lane>>();
    system.add_inbox::<Unbuild, Swarm<Lane>>();
    system.add_inbox::<ConfirmDisconnect, Swarm<Lane>>();

    system.add_individual(Swarm::<TransferLane>::new());
    system.add_inbox::<CreateWith<TransferLane, AdvertiseToTransferAndReport>, Swarm<TransferLane>>();
    system.add_inbox::<AddCar, Swarm<TransferLane>>();
    system.add_inbox::<AddObstacles, Swarm<TransferLane>>();
    system.add_inbox::<Tick, Swarm<TransferLane>>();
    system.add_inbox::<ConnectTransferToNormal, Swarm<TransferLane>>();
    system.add_inbox::<Disconnect, Swarm<TransferLane>>();
    system.add_inbox::<Unbuild, Swarm<TransferLane>>();
    system.add_inbox::<ConfirmDisconnect, Swarm<TransferLane>>();

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
    transfer_acceleration: f32,
    cancelling: bool
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
    Next{green: bool},
    Previous
}
use self::InteractionKind::{Overlap, Next, Previous};

#[derive(Copy, Clone, Debug)]
enum OverlapKind{Parallel, Transfer, Conflicting}
use self::OverlapKind::{Parallel, Transfer, Conflicting};